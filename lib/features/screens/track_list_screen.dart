import 'dart:async';

import 'package:flutter/gestures.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:karbeat/features/components/context_menu.dart';
import 'package:karbeat/features/components/midi_drawer.dart';
import 'package:karbeat/features/components/waveform_painter.dart';
import 'package:karbeat/features/playlist/clip_drag_controller.dart';
import 'package:karbeat/features/playlist/playhead.dart';
import 'package:karbeat/models/interaction_target.dart';
import 'package:karbeat/src/rust/api/plugin.dart' show UiPluginInfo;
import 'package:karbeat/src/rust/api/project.dart';
import 'package:karbeat/src/rust/api/track.dart';
import 'package:karbeat/state/app_state.dart';
import 'package:karbeat/state/clip_placement_state.dart';
import 'package:karbeat/utils/color.dart';
import 'package:karbeat/utils/logger.dart';
import 'package:karbeat/utils/result_type.dart';
import 'package:karbeat/utils/scroll_behavior.dart';
import 'package:linked_scroll_controller/linked_scroll_controller.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

class TrackListScreen extends ConsumerWidget {
  const TrackListScreen({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    return LayoutBuilder(
      builder: (context, constraints) {
        final parentHeight = constraints.maxHeight;
        if (parentHeight.isInfinite) return const SizedBox();

        final calculatedHeight = parentHeight * 0.15;
        final double itemHeight = calculatedHeight.clamp(60.0, 150.0);
        const double headerWidth = 220.0;

        return Builder(
          builder: (context) {
            final trackIdsStr = ref.watch(
              karbeatStateProvider.select((s) {
                final keys = s.tracks.keys.toList()..sort();
                return keys.join(',');
              }),
            );

            final trackIds = trackIdsStr.isEmpty
                ? <int>[]
                : trackIdsStr.split(',').map(int.parse).toList();

            return _SplitTrackView(
              trackIds: trackIds,
              itemHeight: itemHeight,
              headerWidth: headerWidth,
            );
          },
        );
      },
    );
  }
}

class _SplitTrackView extends ConsumerStatefulWidget {
  final List<int> trackIds;
  final double itemHeight;
  final double headerWidth;

  const _SplitTrackView({
    required this.trackIds,
    required this.itemHeight,
    required this.headerWidth,
  });

  @override
  ConsumerState<_SplitTrackView> createState() => _SplitTrackViewState();
}

class _SplitTrackViewState extends ConsumerState<_SplitTrackView> {
  late LinkedScrollControllerGroup _verticalControllers;
  late ScrollController _headerController;
  late ScrollController _timelineController;

  // Horizontal Scrolling (Ruler <-> Tracks)
  late LinkedScrollControllerGroup _horizontalControllers;
  late ScrollController _rulerController; // Controller 1: Top Ruler
  late ScrollController _trackContentController; // Controller 2: Bottom Content

  // Local state for ghost clip
  Offset? _mousePos;

  // LocalState for width
  double _timelineWidth = 2000.0;

  int _activeSampleRate = 44100;

  // ignore:unused_field
  StreamSubscription? _posSub;

  bool _isCtrlPressed = false;

  // Range selection state
  bool _isRangeSelecting = false;
  Offset? _rangeSelectStart; // Position in absolute pixels (including scroll)
  Offset? _rangeSelectEnd;
  int? _rangeSelectTrackId; // Track ID where the range selection started

  // ==========================================================================
  // BATCH CLIP DRAG STATE (centralized for cross-track coordination)
  // ==========================================================================
  final ClipDragController _clipDragController = ClipDragController();

  @override
  void initState() {
    super.initState();
    // Initialize the Linked Group
    _verticalControllers = LinkedScrollControllerGroup();
    _headerController = _verticalControllers.addAndGet();
    _timelineController = _verticalControllers.addAndGet();
    _horizontalControllers = LinkedScrollControllerGroup();
    _rulerController = _horizontalControllers.addAndGet();
    _trackContentController = _horizontalControllers.addAndGet();
    _trackContentController.addListener(_handleScrollExpansion);
    HardwareKeyboard.instance.addHandler(_handleKeyEvents);

    final state = ref.read(karbeatStateProvider);
    _activeSampleRate = state.hardwareConfig.sampleRate > 0
        ? state.hardwareConfig.sampleRate
        : 44100;

    _posSub = state.positionStream.listen((pos) {
      if (!mounted) return;
      if (pos.sampleRate > 0 && pos.sampleRate != _activeSampleRate) {
        // Only setState if it changed to avoid spamming rebuilds
        setState(() {
          _activeSampleRate = pos.sampleRate;
        });
      }
    });

    // Listen to batch drag controller for overlay updates
    _clipDragController.addListener(_onBatchDragUpdate);
  }

  @override
  void dispose() {
    _clipDragController.removeListener(_onBatchDragUpdate);
    _clipDragController.dispose();
    _trackContentController.removeListener(_handleScrollExpansion);
    _headerController.dispose();
    _timelineController.dispose();
    _rulerController.dispose();
    _trackContentController.dispose();
    super.dispose();
  }

  /// Called when batch drag controller updates - triggers overlay repaint
  void _onBatchDragUpdate() {
    if (mounted) {
      setState(() {});
    }
  }

  bool _handleKeyEvents(KeyEvent event) {
    final isCtrl =
        HardwareKeyboard.instance.logicalKeysPressed.contains(
          LogicalKeyboardKey.controlLeft,
        ) ||
        HardwareKeyboard.instance.logicalKeysPressed.contains(
          LogicalKeyboardKey.controlRight,
        );

    if (isCtrl != _isCtrlPressed) {
      // Check mounted before setState in case of fast dispose
      if (mounted) {
        setState(() {
          _isCtrlPressed = isCtrl;
        });
      }
    }
    return false;
  }

  void _handleScrollExpansion() {
    // If the user scrolls within 500px of the edge...
    final maxScroll = _trackContentController.position.maxScrollExtent;
    final currentScroll = _trackContentController.offset;

    if (currentScroll >= maxScroll - 500) {
      // ... Add more space (e.g., another 2000px)
      setState(() {
        _timelineWidth += 2000.0;
      });
    }
  }

  void _updateZoom(double newZoom, double focalPointX) {
    final state = ref.read(karbeatStateProvider);
    final oldZoom = state.horizontalZoomLevel;

    final clampedZoom = newZoom.clamp(0.01, 5000.0);
    if (clampedZoom == oldZoom) return;

    double currentScroll = 0;
    if (_trackContentController.hasClients) {
      currentScroll = _trackContentController.offset;
    }

    // 1. Calculate the exact time (in samples) located at the current cursor point
    final double samplesAtFocalPoint = (currentScroll + focalPointX) * oldZoom;

    // 2. Set the new zoom level
    state.horizontalZoomLevel = clampedZoom;

    // 3. Calculate what the new scroll position MUST be to keep that specific sample at the focal point
    double newScroll = (samplesAtFocalPoint / clampedZoom) - focalPointX;
    if (newScroll < 0) newScroll = 0;

    // 4. Proactively expand the timeline boundary if we zoom in so deep that we pass it
    if (newScroll > _timelineWidth - 1000) {
      setState(() {
        _timelineWidth = newScroll + 2000.0;
      });
      // Wait for layout rebuild to register the new width before jumping
      WidgetsBinding.instance.addPostFrameCallback((_) {
        if (_trackContentController.hasClients) {
          _trackContentController.jumpTo(newScroll);
        }
      });
    } else {
      // Jump immediately
      if (_trackContentController.hasClients) {
        _trackContentController.jumpTo(newScroll);
      }
    }
  }

  void _handleTimelineGesture(
    BuildContext context,
    Offset localPosition, {
    bool isDrag = false,
  }) {
    final state = ref.read(karbeatStateProvider);
    double scrollX = 0;
    if (_trackContentController.hasClients) {
      scrollX = _trackContentController.offset;
    }
    final double absoluteX = localPosition.dx + scrollX;
    if (absoluteX < 0) return;

    switch (state.selectedTool) {
      case ToolSelection.zoom:
        break;
      case ToolSelection.draw:
        setState(() {
          _mousePos = localPosition;
        });
        _updatePlacementTarget();
        break;
      case ToolSelection.pointer:
      case ToolSelection.cut:
      default:
        break;
    }
  }

  /// Starts a range selection when select tool is active
  void _startRangeSelect(Offset localPosition) {
    // Calculate absolute position (including scroll)
    double scrollX = 0;
    double scrollY = 0;
    if (_trackContentController.hasClients) {
      scrollX = _trackContentController.offset;
    }
    if (_timelineController.hasClients) {
      scrollY = _timelineController.offset;
    }

    final absoluteX = localPosition.dx + scrollX;
    final absoluteY = localPosition.dy + scrollY;

    // Determine which track the selection starts on
    int trackIndex = (absoluteY / widget.itemHeight).floor();
    trackIndex = trackIndex.clamp(0, widget.trackIds.length - 1);

    setState(() {
      _isRangeSelecting = true;
      _rangeSelectStart = Offset(absoluteX, absoluteY);
      _rangeSelectEnd = Offset(absoluteX, absoluteY);
      _rangeSelectTrackId = widget.trackIds[trackIndex];
    });
  }

  /// Updates the range selection rectangle during drag
  void _updateRangeSelect(Offset localPosition) {
    if (!_isRangeSelecting || _rangeSelectStart == null) return;

    double scrollX = 0;
    double scrollY = 0;
    if (_trackContentController.hasClients) {
      scrollX = _trackContentController.offset;
    }
    if (_timelineController.hasClients) {
      scrollY = _timelineController.offset;
    }

    final absoluteX = localPosition.dx + scrollX;
    final absoluteY = localPosition.dy + scrollY;

    setState(() {
      _rangeSelectEnd = Offset(absoluteX, absoluteY);
    });
  }

  /// Confirms the range selection and selects all clips within the time range
  void _confirmRangeSelect(KarbeatState state) {
    if (!_isRangeSelecting ||
        _rangeSelectStart == null ||
        _rangeSelectEnd == null ||
        _rangeSelectTrackId == null) {
      _cancelRangeSelect();
      return;
    }

    final zoomLevel = state.horizontalZoomLevel;

    // Get time range in samples
    final startX = _rangeSelectStart!.dx;
    final endX = _rangeSelectEnd!.dx;
    final minX = startX < endX ? startX : endX;
    final maxX = startX > endX ? startX : endX;

    final startTimeSamples = (minX * zoomLevel).toInt();
    final endTimeSamples = (maxX * zoomLevel).toInt();

    // Find clips in the target track that overlap with the selection range
    final track = state.tracks[_rangeSelectTrackId!];
    if (track == null) {
      _cancelRangeSelect();
      return;
    }

    final selectedClipIds = <int>[];
    for (final clip in track.clips) {
      final clipStart = clip.startTime.toInt();
      final clipEnd = clipStart + clip.loopLength.toInt();

      // Check if clip overlaps with selection range
      if (clipEnd > startTimeSamples && clipStart < endTimeSamples) {
        selectedClipIds.add(clip.id);
      }
    }

    // Select the clips
    if (selectedClipIds.isNotEmpty) {
      state.selectClips(
        trackId: _rangeSelectTrackId!,
        clipIds: selectedClipIds,
      );
    } else {
      state.deselectAllClips();
    }

    _cancelRangeSelect();
  }

  /// Cancels/resets the range selection state
  void _cancelRangeSelect() {
    setState(() {
      _isRangeSelecting = false;
      _rangeSelectStart = null;
      _rangeSelectEnd = null;
      _rangeSelectTrackId = null;
    });
  }

  /// Helper method to build the cut helper line
  Widget _buildCutHelperLine(BuildContext context, KarbeatState state) {
    if (_mousePos == null || state.selectedTool != ToolSelection.cut) {
      return const SizedBox();
    }

    double scrollX = 0;
    if (_trackContentController.hasClients) {
      scrollX = _trackContentController.offset;
    }

    double absoluteX = _mousePos!.dx + scrollX;
    if (absoluteX < 0) absoluteX = 0;

    final zoomLevel = state.horizontalZoomLevel;
    double samples = absoluteX * zoomLevel;

    // Apply Snapping
    if (state.snapToGrid) {
      samples = _snapTime(samples.toInt(), state).toDouble();
    }

    double snappedAbsoluteX = samples / zoomLevel;
    double left = widget.headerWidth + (snappedAbsoluteX - scrollX);

    // Hide if scrolled out of view to the left
    if (left < widget.headerWidth) return const SizedBox();

    return Positioned(
      left: left - 12, // Center the 24px wide column exactly on the cut point
      top: 0,
      bottom: 0,
      width: 24,
      child: IgnorePointer(
        child: Column(
          children: [
            const SizedBox(height: 10), // Padding above ruler
            const Icon(Icons.content_cut, color: Colors.redAccent, size: 16),
            Expanded(
              child: Container(
                width: 1.5,
                color: Colors.redAccent.withAlpha(200),
              ),
            ),
          ],
        ),
      ),
    );
  }

  @override
  Widget build(BuildContext context) {
    // Calculate total height to ensure both lists have exactly same extent
    // +1 for the Add Button row
    final int itemCount = widget.trackIds.length + 1;
    final state = ref.read(karbeatStateProvider);
    final isPlacing = ref.watch(
      clipPlacementProvider.select((s) => s.isPlacing),
    );
    final selectedTool = ref.watch(
      karbeatStateProvider.select((s) => s.selectedTool),
    );
    final horizontalZoom = ref.watch(
      karbeatStateProvider.select((s) => s.horizontalZoomLevel),
    );
    final currentTimelineWidth = _timelineWidth;

    handleCursor() {
      if (isPlacing) {
        return SystemMouseCursors.move;
      }

      if (selectedTool == ToolSelection.select) {
        return SystemMouseCursors.precise;
      }

      if (selectedTool == ToolSelection.cut) {
        return SystemMouseCursors.text;
      }

      return SystemMouseCursors.basic;
    }

    return Stack(
      children: [
        Row(
          children: [
            // ======== LEFT: TRACK HEADERS ==========
            SizedBox(
              width: widget.headerWidth,
              child: Column(
                children: [
                  Container(
                    height: 30,
                    color: Colors.grey.shade800,
                    alignment: Alignment.centerLeft,
                    padding: const EdgeInsets.only(left: 10),
                    child: const Text(
                      "Tracks",
                      style: TextStyle(color: Colors.white70, fontSize: 12),
                    ),
                  ),
                  Expanded(
                    child: ListView.builder(
                      controller: _headerController, // Controller 1
                      padding: EdgeInsets.zero,
                      itemCount: itemCount,
                      itemBuilder: (context, index) {
                        if (index == widget.trackIds.length) {
                          return _buildAddButton();
                        }
                        return _TrackHeader(
                          trackId: widget.trackIds[index],
                          itemHeight: widget.itemHeight,
                        );
                      },
                    ),
                  ),
                ],
              ),
            ),

            Container(width: 1, color: Colors.black),

            // ============ RIGHT: TIMELINE ==============
            Expanded(
              child: MouseRegion(
                onHover: (event) {
                  // Placement preview is updated by tap / pan / ghost drag only — not
                  // hover — to avoid the ghost following the cursor and flickering.
                  if (selectedTool == ToolSelection.cut ||
                      selectedTool == ToolSelection.draw) {
                    setState(() => _mousePos = event.localPosition);
                  }
                },
                onExit: (_) {
                  if (isPlacing) return;
                  if (_mousePos != null) {
                    setState(() => _mousePos = null);
                  }
                },
                child: Listener(
                  onPointerSignal: (event) {
                    if (event is PointerScrollEvent) {
                      if (_isCtrlPressed) {
                        final currentZoom = ref
                            .read(karbeatStateProvider)
                            .horizontalZoomLevel;
                        final double multiplier = event.scrollDelta.dy > 0
                            ? 0.9
                            : 1.1;
                        _updateZoom(
                          currentZoom * multiplier,
                          event.localPosition.dx,
                        );
                      }
                    }
                  },
                  child: GestureDetector(
                    behavior: HitTestBehavior.translucent,
                    onTapDown: (details) => _handleTimelineGesture(
                      context,
                      details.localPosition,
                      isDrag: false,
                    ),
                    onPanStart: (details) {
                      // Start range selection when select tool is active
                      if (selectedTool == ToolSelection.select) {
                        _startRangeSelect(details.localPosition);
                      }
                    },
                    onPanUpdate: (details) {
                      // Handle range selection updates
                      if (selectedTool == ToolSelection.select) {
                        _updateRangeSelect(details.localPosition);
                        return;
                      }
                      if (selectedTool == ToolSelection.zoom) {
                        final currentZoom = state.horizontalZoomLevel;
                        double multiplier = 1.0 - (details.delta.dy * 0.01);
                        _updateZoom(
                          currentZoom * multiplier,
                          details.localPosition.dx,
                        );
                        return;
                      }
                      if (selectedTool == ToolSelection.draw || isPlacing) {
                        setState(() => _mousePos = details.localPosition);
                        _updatePlacementTarget();
                      }
                    },
                    onPanEnd: (details) {
                      // Confirm range selection when select tool is active
                      if (selectedTool == ToolSelection.select &&
                          _isRangeSelecting) {
                        _confirmRangeSelect(state);
                      }
                    },
                    child: Column(
                      children: [
                        GestureDetector(
                          onTapDown: (details) {
                            double scrollX = _rulerController.hasClients
                                ? _rulerController.offset
                                : 0;
                            double absoluteX =
                                details.localPosition.dx + scrollX;
                            final samples =
                                absoluteX * state.horizontalZoomLevel;
                            state.seekTo(samples.toInt());
                          },
                          onPanUpdate: (details) {
                            double scrollX = _rulerController.hasClients
                                ? _rulerController.offset
                                : 0;
                            double absoluteX =
                                details.localPosition.dx + scrollX;
                            final samples =
                                absoluteX * state.horizontalZoomLevel;
                            state.seekTo(samples.toInt());
                          },
                          child: Container(
                            height: 30,
                            color: Colors.grey.shade800,
                            width: double.infinity,
                            child: SingleChildScrollView(
                              scrollDirection: Axis.horizontal,
                              controller: _rulerController,
                              physics: _isCtrlPressed
                                  ? const NeverScrollableScrollPhysics()
                                  : const ClampingScrollPhysics(),
                              child: SizedBox(
                                width: currentTimelineWidth,
                                height: 30,
                                child: _TimelineRuler(
                                  scrollController: _rulerController,
                                  sampleRate: _activeSampleRate,
                                ),
                              ),
                            ),
                          ),
                        ),
                        Expanded(
                          child: MouseRegion(
                            cursor: handleCursor(),
                            onHover: null,
                            child: GestureDetector(
                              behavior: HitTestBehavior.translucent,
                              onPanUpdate: null,
                              onTapDown: isPlacing
                                  ? (details) {
                                      setState(() {
                                        _mousePos = details.localPosition;
                                      });
                                      _updatePlacementTarget();
                                    }
                                  : null,
                              child: ScrollConfiguration(
                                // Only allow Mouse Drag scrolling when using Pointer
                                behavior:
                                    (selectedTool == ToolSelection.pointer)
                                    ? DragScrollBehavior()
                                    : ScrollConfiguration.of(context).copyWith(
                                        dragDevices: {
                                          PointerDeviceKind.touch,
                                          PointerDeviceKind.trackpad,
                                        },
                                      ),
                                child: Scrollbar(
                                  controller: _trackContentController,
                                  thumbVisibility: true,
                                  trackVisibility: true,
                                  child: SingleChildScrollView(
                                    scrollDirection: Axis.horizontal,
                                    controller: _trackContentController,
                                    physics: _isCtrlPressed
                                        ? const NeverScrollableScrollPhysics()
                                        : const ClampingScrollPhysics(),
                                    child: SizedBox(
                                      width: currentTimelineWidth,
                                      child: ListView.builder(
                                        controller:
                                            _timelineController, // Controller 2 (Synced Vertically)
                                        physics: _isCtrlPressed
                                            ? const NeverScrollableScrollPhysics()
                                            : const ClampingScrollPhysics(),
                                        padding: EdgeInsets.zero,
                                        itemCount: itemCount,
                                        itemBuilder: (context, index) {
                                          if (index == widget.trackIds.length) {
                                            return SizedBox(height: 60);
                                          }
                                          return IgnorePointer(
                                            ignoring: isPlacing,
                                            child: KarbeatTrackSlot(
                                              trackId: widget.trackIds[index],
                                              height: widget.itemHeight,
                                              horizontalScrollController:
                                                  _trackContentController,
                                              sampleRate: _activeSampleRate,
                                              clipDragController:
                                                  _clipDragController,
                                            ),
                                          );
                                        },
                                      ),
                                    ),
                                  ),
                                ),
                              ),
                            ),
                          ),
                        ),
                      ],
                    ),
                  ),
                ),
              ),
            ),
          ],
        ),
        if (isPlacing && _mousePos != null) _buildGhostClip(context),
        if (_isRangeSelecting) _buildRangeSelectRect(context),
        _buildCutHelperLine(context, state),

        // Batch drag overlays for all selected clips during move
        _GroupedBatchOverlay(
          trackIds: widget.trackIds,
          headerWidth: widget.headerWidth,
          itemHeight: widget.itemHeight,
          horizontalScrollController: _trackContentController,
          timelineController: _timelineController,
          clipDragController: _clipDragController,
        ),
        Positioned.fill(
          child: IgnorePointer(
            ignoring: false,
            child: PlayheadOverlay(
              offsetAdjustment: widget.headerWidth,
              scrollController: _trackContentController,
              zoomLevel: horizontalZoom,
              sampleSelector: (pos) => pos.samples,
              onSeek: (int newSamples) {
                final safeSamples = newSamples < 0 ? 0 : newSamples;
                ref.read(karbeatStateProvider).seekTo(safeSamples);

                KarbeatLogger.info("Seeking to: $safeSamples samples");
              },
            ),
          ),
        ),
        if (isPlacing)
          Positioned(
            bottom: 30,
            right: 30,
            child: Row(
              children: [
                FloatingActionButton.extended(
                  heroTag: 'cancel_place',
                  label: const Text("Cancel"),
                  icon: const Icon(Icons.close),
                  backgroundColor: Colors.redAccent,
                  onPressed: () {
                    setState(() => _mousePos = null);
                    ref.read(clipPlacementProvider.notifier).cancelPlacement();
                  },
                ),
                const SizedBox(width: 16),
                FloatingActionButton.extended(
                  onPressed: () {
                    final messenger = ScaffoldMessenger.of(context);
                    ref
                        .read(clipPlacementProvider.notifier)
                        .confirmPlacement()
                        .then((value) {
                          if (!mounted) return;
                          switch (value) {
                            case Ok<void>():
                              if (!ref.read(clipPlacementProvider).isPlacing) {
                                setState(() => _mousePos = null);
                              }
                              break;
                            case Error<void>():
                              messenger.showSnackBar(
                                SnackBar(content: Text(value.toErrorMessage())),
                              );
                          }
                        });
                  },
                  label: const Text('Confirm'),
                  heroTag: 'confirm_place',
                  icon: Icon(Icons.check),
                  backgroundColor: Colors.greenAccent,
                ),
              ],
            ),
          ),
      ],
    );
  }

  void _updatePlacementTarget() {
    if (_mousePos == null) return;

    // Calculate Absolute Y (Mouse + Scroll)
    double scrollY = 0;
    if (_timelineController.hasClients) {
      scrollY = _timelineController.offset;
    }
    double absoluteY = _mousePos!.dy + scrollY;

    // Determine Track Index
    int trackIndex = (absoluteY / widget.itemHeight).floor();
    trackIndex = trackIndex.clamp(0, widget.trackIds.length - 1);
    final targetTrack = widget.trackIds[trackIndex];

    // Calculate Absolute X (Mouse + Scroll)
    double scrollX = 0;
    if (_trackContentController.hasClients) {
      scrollX = _trackContentController.offset;
    }
    double absoluteX = (_mousePos!.dx + scrollX).clamp(0, double.infinity);

    // Convert X Pixels -> Samples
    final state = ref.read(karbeatStateProvider);
    final zoomLevel = state.horizontalZoomLevel;
    double samples = absoluteX * zoomLevel;

    if (state.snapToGrid) {
      samples = _snapTime(samples.toInt(), state).toDouble();
    }
    ref
        .read(clipPlacementProvider.notifier)
        .updatePlacementTarget(targetTrack, samples);
  }

  Widget _buildGhostClip(BuildContext context) {
    // We map the absolute coordinates back to screen coordinates
    // This is essentially just drawing where the mouse is, but snapped to rows

    // We need logic to snap the ghost Y to the row, but let X float
    // Get current Scroll Offset Y to align grid
    double scrollY = 0;
    if (_timelineController.hasClients) {
      scrollY = _timelineController.offset;
    }
    double absoluteY = _mousePos!.dy + scrollY;
    int trackIndex = (absoluteY / widget.itemHeight).floor();
    trackIndex = trackIndex.clamp(0, widget.trackIds.length - 1);

    double topY = (trackIndex * widget.itemHeight) - scrollY;

    // Offset by header height (approx) + Header Row
    topY += 30;

    double scrollX = 0;
    if (_trackContentController.hasClients) {
      scrollX = _trackContentController.offset;
    }

    final state = ref.read(karbeatStateProvider);
    double absoluteX = _mousePos!.dx + scrollX;
    if (absoluteX < 0) absoluteX = 0;

    double samples = absoluteX * state.horizontalZoomLevel;
    if (state.snapToGrid) {
      samples = _snapTime(samples.toInt(), state).toDouble();
    }

    // Convert the snapped sample position back into a screen pixel coordinate
    double snappedAbsoluteX = samples / state.horizontalZoomLevel;
    double left = widget.headerWidth + (snappedAbsoluteX - scrollX);

    // Safety check to keep it in timeline area
    if (left < widget.headerWidth) left = widget.headerWidth;

    return Positioned(
      left: left,
      top: topY,
      width: 150, // Preview width
      height: widget.itemHeight - 4,
      child: GestureDetector(
        // ENABLE Dragging on the ghost itself
        onPanUpdate: (details) {
          setState(() {
            // Update _mousePos relative to the drag delta
            if (_mousePos != null) {
              _mousePos = _mousePos! + details.delta;
            }
          });
          // Update the logic state
          _updatePlacementTarget();
        },
        child: Opacity(
          opacity: 0.7,
          // REMOVE IgnorePointer so it can catch the Drag events
          child: MouseRegion(
            cursor: SystemMouseCursors.move, // Indicate draggable
            child: Container(
              decoration: BoxDecoration(
                color: Colors.cyanAccent.withAlpha(100),
                border: Border.all(color: Colors.cyanAccent, width: 2),
                borderRadius: BorderRadius.circular(4),
              ),
              child: const Center(
                child: Text(
                  "Place Here",
                  style: TextStyle(
                    color: Colors.white,
                    fontWeight: FontWeight.bold,
                    shadows: [Shadow(color: Colors.black, blurRadius: 2)],
                  ),
                ),
              ),
            ),
          ),
        ),
      ),
    );
  }

  /// Builds the visual rectangle overlay for range selection
  Widget _buildRangeSelectRect(BuildContext context) {
    if (_rangeSelectStart == null ||
        _rangeSelectEnd == null ||
        _rangeSelectTrackId == null) {
      return const SizedBox();
    }

    // Get scroll offsets
    double scrollX = 0;
    double scrollY = 0;
    if (_trackContentController.hasClients) {
      scrollX = _trackContentController.offset;
    }
    if (_timelineController.hasClients) {
      scrollY = _timelineController.offset;
    }

    // Find the track index for the starting track
    final trackIndex = widget.trackIds.indexWhere(
      (t) => t == _rangeSelectTrackId,
    );
    if (trackIndex < 0) return const SizedBox();

    // Calculate the rectangle bounds (only horizontal matters, vertical is fixed to the track)
    final startX = _rangeSelectStart!.dx;
    final endX = _rangeSelectEnd!.dx;
    final minX = startX < endX ? startX : endX;
    final maxX = startX > endX ? startX : endX;

    // Convert from absolute coordinates to screen coordinates
    final screenLeft = minX - scrollX + widget.headerWidth;
    final screenWidth = maxX - minX;

    // Track row position (fixed to the starting track)
    final screenTop =
        (trackIndex * widget.itemHeight) - scrollY + 30; // +30 for ruler height

    return Positioned(
      left: screenLeft,
      top: screenTop,
      width: screenWidth < 2 ? 2 : screenWidth,
      height: widget.itemHeight - 4,
      child: IgnorePointer(
        child: Container(
          decoration: BoxDecoration(
            color: Colors.blueAccent.withAlpha(50),
            border: Border.all(color: Colors.blueAccent, width: 2),
            borderRadius: BorderRadius.circular(4),
          ),
        ),
      ),
    );
  }

  Widget _buildAddButton() {
    return SizedBox(
      height: 60,
      child: Center(
        child: TextButton.icon(
          onPressed: () => _showAddTrackDialog(context),
          icon: const Icon(Icons.add, color: Colors.white54),
          label: const Text(
            "Add New Track",
            style: TextStyle(color: Colors.white54),
          ),
        ),
      ),
    );
  }

  void _showAddTrackDialog(BuildContext context) {
    showDialog(
      context: context,
      builder: (ctx) => SimpleDialog(
        title: const Text("Add New Track"),
        children: [
          SimpleDialogOption(
            onPressed: () {
              Navigator.pop(ctx);
              ref.read(karbeatStateProvider).addAudioTrack();
            },
            child: const Row(
              children: [
                Icon(Icons.graphic_eq, color: Colors.cyanAccent),
                SizedBox(width: 10),
                Text("Audio Track"),
              ],
            ),
          ),
          const Divider(),
          SimpleDialogOption(
            onPressed: () {
              Navigator.pop(ctx);
              _showGeneratorBrowser(context);
            },
            child: const Row(
              children: [
                Icon(Icons.piano, color: Colors.orangeAccent),
                SizedBox(width: 10),
                Text("Add generator..."),
              ],
            ),
          ),
        ],
      ),
    );
  }

  void _showGeneratorBrowser(BuildContext context) {
    final availablePlugins = ref.read(karbeatStateProvider).availableGenerators;

    showDialog(
      context: context,
      builder: (ctx) => AlertDialog(
        title: const Text("Generator Browser"),
        contentPadding: const EdgeInsets.only(top: 12, bottom: 24),
        content: SizedBox(
          width: 360,
          child: Column(
            mainAxisSize: MainAxisSize.min,
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              // Category header: Karbeat Native
              Padding(
                padding: const EdgeInsets.symmetric(
                  horizontal: 24,
                  vertical: 8,
                ),
                child: Row(
                  children: [
                    Icon(
                      Icons.extension,
                      size: 16,
                      color: Colors.deepOrangeAccent,
                    ),
                    const SizedBox(width: 8),
                    Container(
                      padding: const EdgeInsets.symmetric(
                        horizontal: 8,
                        vertical: 3,
                      ),
                      decoration: BoxDecoration(
                        color: Colors.deepOrangeAccent.withAlpha(30),
                        borderRadius: BorderRadius.circular(4),
                        border: Border.all(
                          color: Colors.deepOrangeAccent.withAlpha(80),
                        ),
                      ),
                      child: const Text(
                        "Karbeat Native",
                        style: TextStyle(
                          color: Colors.deepOrangeAccent,
                          fontSize: 12,
                          fontWeight: FontWeight.w600,
                        ),
                      ),
                    ),
                  ],
                ),
              ),
              const Divider(height: 1),
              // Plugin list
              if (availablePlugins.isEmpty)
                const Padding(
                  padding: EdgeInsets.symmetric(horizontal: 24, vertical: 16),
                  child: Text(
                    "No generators found",
                    style: TextStyle(color: Colors.grey),
                  ),
                )
              else
                ...availablePlugins.map(
                  (plugin) => _buildGeneratorBrowserItem(ctx, plugin),
                ),
            ],
          ),
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(ctx),
            child: const Text("Cancel"),
          ),
        ],
      ),
    );
  }

  Widget _buildGeneratorBrowserItem(BuildContext ctx, UiPluginInfo plugin) {
    return InkWell(
      onTap: () {
        Navigator.pop(ctx);
        ref.read(karbeatStateProvider).addMidiTrackWithGeneratorId(plugin.id);
      },
      child: Padding(
        padding: const EdgeInsets.symmetric(horizontal: 24, vertical: 10),
        child: Row(
          children: [
            const Icon(Icons.piano, color: Colors.orangeAccent, size: 20),
            const SizedBox(width: 12),
            Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text(
                    plugin.name,
                    style: const TextStyle(
                      fontSize: 14,
                      fontWeight: FontWeight.w500,
                    ),
                  ),
                  const SizedBox(height: 2),
                  const Text(
                    "Karbeat Native",
                    style: TextStyle(fontSize: 11, color: Colors.grey),
                  ),
                ],
              ),
            ),
          ],
        ),
      ),
    );
  }
}

class _TrackHeader extends ConsumerWidget {
  final int trackId;
  final double itemHeight;

  const _TrackHeader({required this.trackId, required this.itemHeight});

  Color _getContrastColor(Color backgroundColor) {
    return backgroundColor.computeLuminance() > 0.5
        ? Colors.black
        : Colors.white;
  }

  IconData _getTrackIcon(UiTrackType type) {
    switch (type) {
      case UiTrackType.audio:
        return Icons.graphic_eq;
      case UiTrackType.midi:
        return Icons.piano;
      case UiTrackType.automation:
        return Icons.show_chart;
    }
  }

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    // Only rebuilds this specific header if the track's name/color/type changes
    final track = ref.watch(
      karbeatStateProvider.select((s) => s.tracks[trackId]),
    );

    if (track == null) return const SizedBox();

    return ContextMenuWrapper(
      title: track.name,
      header: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text(
            "Name: ${track.name}",
            style: const TextStyle(color: Colors.white70, fontSize: 13),
          ),
          const SizedBox(height: 4),
          Text(
            "Type: ${track.trackType.name.toUpperCase()}",
            style: const TextStyle(color: Colors.white70, fontSize: 13),
          ),
          const SizedBox(height: 4),
          Text(
            "ID: ${track.id}",
            style: const TextStyle(color: Colors.white70, fontSize: 13),
          ),
          const SizedBox(height: 8),
          Row(
            children: [
              const Text(
                "Color: ",
                style: TextStyle(color: Colors.white70, fontSize: 13),
              ),
              Container(
                width: 14,
                height: 14,
                decoration: BoxDecoration(
                  color: Color(
                    int.parse(track.color.substring(1), radix: 16),
                  ), // Replace with track.color if available
                  borderRadius: BorderRadius.circular(2),
                ),
              ),
            ],
          ),
        ],
      ),
      actions: [
        KarbeatContextAction(
          title: "Rename",
          icon: Icons.edit,
          onTap: () {
            // Replace with your actual rename logic via app_state
            KarbeatLogger.info("Rename track requested for ID: ${track.id}");
          },
        ),
        KarbeatContextAction(
          title: "Move Up",
          icon: Icons.arrow_upward,
          onTap: () {
            // Replace with actual move up logic
            KarbeatLogger.info("Move Up requested for track ID: ${track.id}");
          },
        ),
        KarbeatContextAction(
          title: "Move Down",
          icon: Icons.arrow_downward,
          onTap: () {
            // Replace with actual move down logic
            KarbeatLogger.info("Move Down requested for track ID: ${track.id}");
          },
        ),
        KarbeatContextAction(
          title: "Delete Track",
          icon: Icons.delete,
          isDestructive: true,
          onTap: () {
            // Replace with actual delete logic via app_state
            KarbeatLogger.info("Delete track requested for ID: ${track.id}");
          },
        ),
      ],
      child: SizedBox(
        height: itemHeight,
        child: Container(
          margin: const EdgeInsets.only(bottom: 2),
          padding: const EdgeInsets.symmetric(horizontal: 10),
          decoration: BoxDecoration(
            color: track.color.toColor(),
            border: Border(
              bottom: BorderSide(color: Colors.grey.shade400, width: 1),
              right: BorderSide(color: Colors.grey.shade400, width: 1),
            ),
          ),
          child: Row(
            children: [
              Icon(_getTrackIcon(track.trackType), color: Colors.grey.shade700),
              const SizedBox(width: 10),
              Expanded(
                child: Column(
                  mainAxisAlignment: MainAxisAlignment.center,
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Text(
                      track.name,
                      style: TextStyle(
                        color: Colors.grey.shade800,
                        fontWeight: FontWeight.bold,
                        fontSize: 14,
                      ),
                      overflow: TextOverflow.ellipsis,
                    ),
                    Text(
                      "ID: ${track.id} | ${track.trackType.name.toUpperCase()}",
                      style: TextStyle(
                        color: _getContrastColor(track.color.toColor()),
                        // color: Colors.grey.shade600, // use inverse color of track color for better contrast
                        fontSize: 10,
                      ),
                    ),
                  ],
                ),
              ),
              Column(
                mainAxisAlignment: MainAxisAlignment.center,
                children: [
                  InkWell(
                    onTap: () {},
                    child: const Icon(
                      Icons.mic_off,
                      size: 16,
                      color: Colors.grey,
                    ),
                  ),
                  const SizedBox(height: 4),
                  InkWell(
                    onTap: () {},
                    child: const Icon(
                      Icons.volume_up,
                      size: 16,
                      color: Colors.grey,
                    ),
                  ),
                ],
              ),
            ],
          ),
        ),
      ),
    );
  }
}

class _TimelineRuler extends ConsumerWidget {
  final ScrollController scrollController;
  final int sampleRate;

  const _TimelineRuler({
    required this.scrollController,
    required this.sampleRate,
  });

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    // Read state for drawing
    final zoomLevel = ref.watch(
      karbeatStateProvider.select((s) => s.horizontalZoomLevel),
    );
    final tempo = ref.watch(karbeatStateProvider.select((s) => s.tempo));
    final safeSampleRate = sampleRate <= 0 ? 48000 : sampleRate;

    return RepaintBoundary(
      child: CustomPaint(
        size: Size.zero,
        painter: _TimelineRulerPainter(
          zoomLevel: zoomLevel,
          tempo: tempo,
          sampleRate: safeSampleRate,
          scrollController: scrollController,
        ),
      ),
    );
  }
}

class _TimelineRulerPainter extends CustomPainter {
  final double zoomLevel;
  final double tempo;
  final int sampleRate;
  final ScrollController scrollController;

  _TimelineRulerPainter({
    required this.zoomLevel,
    required this.tempo,
    required this.sampleRate,
    required this.scrollController,
  }) : super(repaint: scrollController);

  @override
  void paint(Canvas canvas, Size size) {
    if (zoomLevel <= 0 || tempo <= 0 || sampleRate <= 0) return;

    // Calculate Intervals
    final double samplesPerBeat = (60.0 / tempo) * sampleRate;
    final double pixelsPerBeat = samplesPerBeat / zoomLevel;

    if (pixelsPerBeat < 1.0) return;

    // Drawing Settings
    final TextPainter textPainter = TextPainter(
      textDirection: TextDirection.ltr,
    );

    final Paint majorTickPaint = Paint()
      ..color = Colors.white54
      ..strokeWidth = 1.0;

    final Paint minorTickPaint = Paint()
      ..color = Colors.white24
      ..strokeWidth = 1.0;

    const int beatsPerBar = 4;
    final double pixelsPerBar = pixelsPerBeat * beatsPerBar;

    // Calculate Visible Range safely
    double startPixel = 0.0;
    double endPixel = size.width;

    // Handle multiple clients safely
    if (scrollController.hasClients) {
      // When a controller is attached to multiple views, .offset throws.
      // We must access specific positions. Since they are synced, taking the first is fine.
      final position = scrollController.positions.first;

      final offset = position.pixels;
      final viewportWidth = position.hasViewportDimension
          ? position.viewportDimension
          : 1000.0;

      const double buffer = 200.0;
      startPixel = (offset - buffer).clamp(0.0, double.infinity);
      endPixel = offset + viewportWidth + buffer;
    }

    // Determine Start Index
    int barIndex = (startPixel / pixelsPerBar).floor();
    if (barIndex < 1) barIndex = 1;

    double currentX = (barIndex - 1) * pixelsPerBar;

    // Draw Loop
    while (currentX < endPixel) {
      if (currentX > size.width) break;

      if (currentX >= startPixel) {
        // Draw Major Tick
        canvas.drawLine(
          Offset(currentX, 15),
          Offset(currentX, size.height),
          majorTickPaint,
        );

        // Draw Bar Number
        textPainter.text = TextSpan(
          text: '$barIndex',
          style: const TextStyle(color: Colors.white70, fontSize: 10),
        );
        textPainter.layout();
        textPainter.paint(canvas, Offset(currentX + 4, 2));
      }

      // Draw Beat Ticks
      if (pixelsPerBeat > 5.0) {
        for (int i = 1; i < beatsPerBar; i++) {
          double beatX = currentX + (pixelsPerBeat * i);

          if (beatX >= startPixel && beatX < endPixel && beatX < size.width) {
            canvas.drawLine(
              Offset(beatX, 22),
              Offset(beatX, size.height),
              minorTickPaint,
            );
          }
        }
      }

      currentX += pixelsPerBar;
      barIndex++;
    }
  }

  @override
  bool shouldRepaint(covariant _TimelineRulerPainter oldDelegate) {
    return oldDelegate.zoomLevel != zoomLevel ||
        oldDelegate.tempo != tempo ||
        oldDelegate.sampleRate != sampleRate ||
        oldDelegate.scrollController != scrollController;
  }
}

class KarbeatTrackSlot extends ConsumerStatefulWidget {
  final int trackId;
  final double height;
  final ScrollController horizontalScrollController;
  final int sampleRate;
  final ClipDragController clipDragController;

  const KarbeatTrackSlot({
    super.key,
    required this.trackId,
    required this.height,
    required this.horizontalScrollController,
    required this.sampleRate,
    required this.clipDragController,
  });

  @override
  ConsumerState<KarbeatTrackSlot> createState() => _KarbeatTrackSlotState();
}

class _KarbeatTrackSlotState extends ConsumerState<KarbeatTrackSlot> {
  void _handleEmptySpaceClick({
    required BuildContext context,
    required double localDx,
    required double zoomLevel,
  }) {
    final state = ref.read(karbeatStateProvider);
    int startTime = (localDx * zoomLevel).round();

    if (state.snapToGrid) {
      startTime = _snapTime(startTime, state);
    }

    state.createEmptyPatternClip(trackId: widget.trackId, startTime: startTime);
  }

  @override
  Widget build(BuildContext context) {
    // Listen to Zoom Level (Global)
    final zoomLevel = ref.watch(
      karbeatStateProvider.select((s) => s.horizontalZoomLevel),
    );

    final gridSize = ref.watch(karbeatStateProvider.select((s) => s.gridSize));
    final tempo = ref.watch(karbeatStateProvider.select((s) => s.tempo));

    // Listen to Track Data
    final track = ref.watch(
      karbeatStateProvider.select((s) => s.tracks[widget.trackId]),
    );

    final isSelectedTrack = ref.watch(
      karbeatStateProvider.select((s) => s.selectedTrackId == widget.trackId),
    );

    final trackSelectedClipIdsStr = ref.watch(
      karbeatStateProvider.select((s) {
        if (s.selectedTrackId != widget.trackId) return '';
        return s.selectedClipIds.join(',');
      }),
    );

    final safeSampleRate = widget.sampleRate <= 0 ? 48000 : widget.sampleRate;

    final waveformMapAsync = ref.watch(
      trackWaveformProvider((trackId: widget.trackId)),
    );

    final selectedTool = ref.watch(
      karbeatStateProvider.select((s) => s.selectedTool),
    );

    final selectedClipIds = trackSelectedClipIdsStr.isEmpty
        ? <int>[]
        : trackSelectedClipIdsStr.split(',').map(int.parse).toList();

    if (track == null) return const SizedBox();

    return Container(
      height: widget.height,
      decoration: BoxDecoration(
        border: Border(
          bottom: BorderSide(color: Colors.white.withAlpha(16), width: 1),
          right: BorderSide(color: Colors.white.withAlpha(16), width: 1),
        ),
        color: Colors.grey.shade900,
      ),
      child: Stack(
        clipBehavior: Clip.none, // Allow clips to drag outside temporarily
        children: [
          Positioned.fill(
            child: MouseRegion(
              cursor: selectedTool == ToolSelection.draw
                  ? SystemMouseCursors.precise
                  : SystemMouseCursors.basic,
              child: GestureDetector(
                behavior: HitTestBehavior.translucent,
                onTapUp: (details) {
                  if (selectedTool == ToolSelection.draw) {
                    _handleEmptySpaceClick(
                      context: context,
                      localDx: details.localPosition.dx,
                      zoomLevel: zoomLevel,
                    );
                  } else {
                    ref.read(karbeatStateProvider).deselectAllClips();
                  }
                },
                child: RepaintBoundary(
                  child: CustomPaint(
                    painter: _GridPainter(
                      zoomLevel: zoomLevel,
                      gridSize: gridSize,
                      tempo: tempo,
                      sampleRate: safeSampleRate,
                      scrollController: widget.horizontalScrollController,
                    ),
                  ),
                ),
              ),
            ),
          ),
          ...waveformMapAsync.when(
            loading: () => [
              const Positioned.fill(
                child: Center(child: CircularProgressIndicator()),
              ),
            ],

            error: (err, _) => [
              Positioned.fill(
                child: Center(
                  child: Text(
                    "Error loading waveforms",
                    style: TextStyle(color: Colors.red),
                  ),
                ),
              ),
            ],

            data: (waveformMap) {
              return track.clips.map((clip) {
                final isSelected =
                    isSelectedTrack && selectedClipIds.contains(clip.id);

                return _InteractiveClip(
                  key: ValueKey(clip.id),
                  clip: clip,
                  trackId: widget.trackId,
                  trackType: track.trackType,
                  zoomLevel: zoomLevel,
                  height: widget.height,
                  selectedTool: selectedTool,
                  isSelected: isSelected,
                  selectedClipIds: selectedClipIds,
                  clipDragController: widget.clipDragController,
                  horizontalScrollController: widget.horizontalScrollController,

                  // ✅ NEW
                  waveformMap: waveformMap,
                );
              }).toList();
            },
          ),
        ],
      ),
    );
  }
}

// =============================================================================
// INTERACTIVE CLIP WRAPPER (Handles Logic)
// =============================================================================

class _InteractiveClip extends ConsumerStatefulWidget {
  final UiClip clip;
  final int trackId;
  final UiTrackType trackType;
  final double zoomLevel;
  final double height;
  final ToolSelection selectedTool;
  final bool isSelected;
  final List<int> selectedClipIds;
  final ClipDragController clipDragController;
  final ScrollController horizontalScrollController;
  final Map<int, AudioWaveformUiForClip> waveformMap;

  const _InteractiveClip({
    super.key,
    required this.clip,
    required this.trackId,
    required this.trackType,
    required this.zoomLevel,
    required this.height,
    required this.selectedTool,
    required this.isSelected,
    required this.selectedClipIds,
    required this.clipDragController,
    required this.horizontalScrollController,
    required this.waveformMap,
  });

  @override
  ConsumerState<_InteractiveClip> createState() => _InteractiveClipState();
}

enum _DragAction { none, resizeLeft, resizeRight, move }

class _InteractiveClipState extends ConsumerState<_InteractiveClip> {
  // Local state for smooth UI updates during drag
  late int _visualStartTime;
  late int _visualLoopLength;
  late int _visualOffset;

  _DragAction _currentAction = _DragAction.none;

  // Drag Math Tracking
  double _accumulatedDeltaSamples = 0.0;
  double _verticalDragDy = 0.0;
  int _previousSnappedDelta = 0;
  int _leaderBaseStartTime = 0;
  int _leaderBaseLoopLength = 0;

  // Track vertical drag to determine target track

  /// Track dynamic cursor override
  MouseCursor? _cursorOverride;

  // double _accumulatedDeltaSamples = 0.0;

  @override
  void initState() {
    super.initState();
    _syncModel();
    widget.clipDragController.addListener(_onBatchDragUpdate);
  }

  void _onBatchDragUpdate() {
    if (mounted) setState(() {});
  }

  @override
  void dispose() {
    widget.clipDragController.removeListener(_onBatchDragUpdate);
    super.dispose();
  }

  @override
  void didUpdateWidget(covariant _InteractiveClip oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (_currentAction == _DragAction.none) {
      _syncModel();
    }
  }

  void _syncModel() {
    _visualStartTime = widget.clip.startTime.toInt();
    _visualLoopLength = widget.clip.loopLength.toInt();
    _visualOffset = widget.clip.offsetStart.toInt();
    // Store base values for follower calculations
  }

  @override
  Widget build(BuildContext context) {
    // Coordinate Mapping (Pixels)
    final double left = _visualStartTime / widget.zoomLevel;
    final double width = _visualLoopLength / widget.zoomLevel;
    final double safeWidth = width < 1 ? 1 : width;
    const resizeEdgeSize = 20.0;

    final isMoving = _currentAction == _DragAction.move;

    // final double top = 2 + _verticalDragDy;

    // Determine Cursor
    MouseCursor cursor = SystemMouseCursors.basic;
    if (widget.selectedTool == ToolSelection.delete) {
      cursor = SystemMouseCursors.click;
    } else if (widget.selectedTool == ToolSelection.move) {
      cursor = SystemMouseCursors.move;
    } else if (widget.selectedTool == ToolSelection.resize) {
      cursor = SystemMouseCursors.basic; // Overridden on hover at edges
    }
    // Apply Override
    if (_cursorOverride != null) {
      cursor = _cursorOverride!;
    }

    // Check if this is a follower in a batch move (should be semi-transparent)
    final isPartOfBatchDrag =
        widget.isSelected && widget.clipDragController.isActive;

    return Positioned(
      left: left,
      top: 2,
      height: widget.height - 4,
      width: safeWidth,
      child: Opacity(
        // Leader becomes invisible (has overlay), followers become semi-transparent
        opacity: isPartOfBatchDrag ? 0.3 : 1.0,
        child: IgnorePointer(
          ignoring: widget.clipDragController.isActive,
          child: MouseRegion(
            cursor: cursor,
            // Detect Hover for Resize Cursors (only if not in Delete mode)
            onHover: (event) {
              if (widget.selectedTool != ToolSelection.resize) {
                if (_cursorOverride != null) {
                  setState(() => _cursorOverride = null);
                }
                return;
              }

              final x = event.localPosition.dx;
              if (x < resizeEdgeSize || x > safeWidth - resizeEdgeSize) {
                if (_cursorOverride != SystemMouseCursors.resizeLeftRight) {
                  setState(() {
                    _cursorOverride = SystemMouseCursors.resizeLeftRight;
                  });
                } else {
                  if (_cursorOverride != null) {
                    setState(() {
                      _cursorOverride = null;
                    });
                  }
                }
              }
            },
            onExit: (event) {
              if (_cursorOverride != null) {
                setState(() {
                  _cursorOverride = null;
                });
              }
            },
            child: GestureDetector(
              // Opaque ensures we catch taps even on transparent parts of waveform
              behavior: HitTestBehavior.opaque,

              onTapUp: (details) async {
                if (widget.selectedTool == ToolSelection.delete) {
                  final state = ref.read(karbeatStateProvider);
                  // If this clip is selected and there are multiple selections, batch delete
                  if (widget.isSelected && widget.selectedClipIds.length > 1) {
                    state.deleteSelectedClips();
                  } else {
                    state.deleteClip(widget.trackId, widget.clip.id);
                  }
                } else if (widget.selectedTool == ToolSelection.select) {
                  final state = ref.read(karbeatStateProvider);
                  // Get tap position for panel positioning
                  final renderBox = context.findRenderObject() as RenderBox?;
                  final tapPosition =
                      renderBox?.localToGlobal(Offset.zero) ?? Offset.zero;

                  // If not already selected, select it first
                  if (!widget.isSelected) {
                    state.selectClip(
                      trackId: widget.trackId,
                      clipId: widget.clip.id,
                    );
                  }

                  // Show interaction panel
                  if (widget.isSelected && widget.selectedClipIds.length > 1) {
                    state.showInteractionPanel(
                      MultiClipInteraction(
                        trackId: widget.trackId,
                        clipIds: widget.selectedClipIds,
                        tapPosition: tapPosition,
                      ),
                    );
                  } else {
                    state.showInteractionPanel(
                      ClipInteraction(
                        trackId: widget.trackId,
                        clipId: widget.clip.id,
                        tapPosition: tapPosition,
                      ),
                    );
                  }
                } else if (widget.selectedTool == ToolSelection.pointer) {
                  ref
                      .read(karbeatStateProvider)
                      .selectClip(
                        trackId: widget.trackId,
                        clipId: widget.clip.id,
                      );
                } else if (widget.selectedTool == ToolSelection.cut) {
                  final state = ref.read(karbeatStateProvider);

                  // Calculate absolute sample position on the timeline
                  int cutSample =
                      widget.clip.startTime +
                      (details.localPosition.dx * widget.zoomLevel).round();

                  // Force the cut to match the snapped grid!
                  if (state.snapToGrid) {
                    cutSample = _snapTime(cutSample, state);
                  }

                  final result = await state.cutClip(
                    widget.trackId,
                    widget.clip.id,
                    cutSample,
                  );

                  if (result.isErr()) {
                    // Optional: Show error toast if cut fails (e.g. out of bounds)
                  }
                }
              },

              onPanStart: (details) {
                if (widget.selectedTool != ToolSelection.move &&
                    widget.selectedTool != ToolSelection.resize) {
                  return;
                }

                _accumulatedDeltaSamples = 0.0;
                _verticalDragDy = 0.0;
                _previousSnappedDelta = 0;

                final x = details.localPosition.dx;

                if (widget.selectedTool == ToolSelection.resize) {
                  if (x < resizeEdgeSize) {
                    setState(() => _currentAction = _DragAction.resizeLeft);
                  } else if (x > safeWidth - resizeEdgeSize) {
                    setState(() => _currentAction = _DragAction.resizeRight);
                  } else {
                    // Clicked the middle of the clip with the resize tool -> do nothing
                    return;
                  }
                } else if (widget.selectedTool == ToolSelection.move) {
                  // Move tool grabs the clip no matter where you click
                  setState(() => _currentAction = _DragAction.move);
                }

                // If the user starts dragging an UNSELECTED clip, select it first!
                final state = ref.read(karbeatStateProvider);
                if (!widget.isSelected) {
                  state.selectClip(
                    trackId: widget.trackId,
                    clipId: widget.clip.id,
                  );
                }

                // Find the leader clip (the earliest one) to act as the snapping anchor
                final currentSelectedIds = state.selectedClipIds;
                final track = state.tracks[widget.trackId];
                if (track != null && currentSelectedIds.isNotEmpty) {
                  final leaderClip = track.clips
                      .where((c) => currentSelectedIds.contains(c.id))
                      .reduce((a, b) => a.startTime < b.startTime ? a : b);
                  _leaderBaseStartTime = leaderClip.startTime.toInt();
                  _leaderBaseLoopLength = leaderClip.loopLength.toInt();
                }

                final batchAction = _currentAction == _DragAction.move
                    ? BatchDragAction.move
                    : _currentAction == _DragAction.resizeLeft
                    ? BatchDragAction.resizeLeft
                    : _currentAction == _DragAction.resizeRight
                    ? BatchDragAction.resizeRight
                    : BatchDragAction.none;

                widget.clipDragController.startBatchDrag(batchAction);
              },

              onPanUpdate: (details) {
                if (_currentAction == _DragAction.none) return;

                final state = ref.read(karbeatStateProvider);
                final currentSelectedIds = state.selectedClipIds;
                final track = state.tracks[widget.trackId];
                if (track == null) return;

                _accumulatedDeltaSamples += details.delta.dx * widget.zoomLevel;
                _verticalDragDy += details.delta.dy;

                int rawTotalDelta = _accumulatedDeltaSamples.round();

                // Safety Clamp: Prevent shrinking past 100 samples
                final minSamples = 100;
                final shortestClip = track.clips
                    .where((c) => currentSelectedIds.contains(c.id))
                    .reduce((a, b) => a.loopLength < b.loopLength ? a : b);

                if (_currentAction == _DragAction.resizeRight) {
                  final maxShrink = -(shortestClip.loopLength - minSamples);
                  if (rawTotalDelta < maxShrink)
                    rawTotalDelta = maxShrink.toInt();
                } else if (_currentAction == _DragAction.resizeLeft) {
                  final maxShrink = shortestClip.loopLength - minSamples;
                  if (rawTotalDelta > maxShrink)
                    rawTotalDelta = maxShrink.toInt();
                }

                int snappedTotalDelta = rawTotalDelta;

                // Snapping Logic
                if (state.snapToGrid) {
                  if (_currentAction == _DragAction.move ||
                      _currentAction == _DragAction.resizeLeft) {
                    int rawStart = _leaderBaseStartTime + rawTotalDelta;
                    snappedTotalDelta =
                        _snapTime(rawStart, state) - _leaderBaseStartTime;
                  } else if (_currentAction == _DragAction.resizeRight) {
                    int rawEnd =
                        _leaderBaseStartTime +
                        _leaderBaseLoopLength +
                        rawTotalDelta;
                    snappedTotalDelta =
                        _snapTime(rawEnd, state) -
                        (_leaderBaseStartTime + _leaderBaseLoopLength);
                  }
                }

                _previousSnappedDelta = snappedTotalDelta;

                // PUSH DATA TO THE OVERLAY
                widget.clipDragController.updateDrag(
                  snappedTotalDelta,
                  _verticalDragDy,
                );
              },

              onPanEnd: (_) {
                if (_currentAction == _DragAction.none) return;

                final state = ref.read(karbeatStateProvider);
                final currentSelectedIds = state.selectedClipIds;

                int? newTrackId;
                final rowOffset = (_verticalDragDy / widget.height).round();
                if (rowOffset != 0) {
                  final sortedTracks = state.tracks.keys.toList()..sort();
                  final currentIndex = sortedTracks.indexWhere(
                    (id) => id == widget.trackId,
                  );
                  if (currentIndex != -1) {
                    final targetIndex = currentIndex + rowOffset;
                    if (targetIndex >= 0 && targetIndex < sortedTracks.length) {
                      newTrackId = sortedTracks[targetIndex];
                    }
                  }
                }

                // Send command to Rust backend
                if (_currentAction == _DragAction.move) {
                  state.moveClipBatch(
                    widget.trackId,
                    currentSelectedIds,
                    _previousSnappedDelta,
                    newTrackId: newTrackId,
                  );
                } else if (_currentAction == _DragAction.resizeRight) {
                  state.resizeClipBatch(
                    widget.trackId,
                    currentSelectedIds,
                    UiResizeEdge.right,
                    _previousSnappedDelta,
                  );
                } else if (_currentAction == _DragAction.resizeLeft) {
                  state.resizeClipBatch(
                    widget.trackId,
                    currentSelectedIds,
                    UiResizeEdge.left,
                    _previousSnappedDelta,
                  );
                }

                setState(() {
                  _currentAction = _DragAction.none;
                });

                widget.clipDragController.reset();
              },

              child: _ClipRenderer(
                clip: widget.clip,
                trackType: widget.trackType,
                color: Colors.cyanAccent.withAlpha(47),
                zoomLevel: widget.zoomLevel,
                projectSampleRate: ref
                    .read(karbeatStateProvider)
                    .hardwareConfig
                    .sampleRate,
                overrideOffset: _visualOffset.toDouble(),
                isSelected: widget.isSelected,
                scrollController: widget.horizontalScrollController,
                clipLeftOffset: left,
                waveformMap: widget.waveformMap,
              ),
            ),
          ),
        ),
      ),
    );
  }
}

// =============================================================================
// 2. THE CLIP RENDERER (The actual colored box)
// =============================================================================

class _ClipRenderer extends ConsumerWidget {
  final UiClip clip;
  final UiTrackType trackType;
  final Color color;
  final double zoomLevel;
  final int projectSampleRate;
  final double? overrideOffset;
  final bool isSelected;
  final ScrollController scrollController;
  final double clipLeftOffset;
  final Map<int, AudioWaveformUiForClip> waveformMap;

  const _ClipRenderer({
    required this.clip,
    required this.trackType,
    required this.color,
    required this.zoomLevel,
    required this.projectSampleRate,
    this.overrideOffset,
    required this.isSelected,
    required this.scrollController,
    required this.clipLeftOffset,
    required this.waveformMap,
  });

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    return Container(
      decoration: BoxDecoration(
        color: color,
        borderRadius: BorderRadius.circular(4),
        border: isSelected
            ? Border.all(color: Colors.white, width: 2)
            : Border.all(color: color.withAlpha(16), width: 1),
      ),
      child: ClipRRect(
        borderRadius: BorderRadius.circular(3),
        child: Stack(
          children: [
            // A. Content (Waveform or MIDI Notes)
            Positioned.fill(child: _buildContent(context, ref)),

            // B. Label Header
            Positioned(
              top: 0,
              left: 0,
              right: 0,
              height: 16,
              child: Container(
                padding: const EdgeInsets.symmetric(horizontal: 4),
                color: Colors.black26,
                child: Text(
                  clip.name,
                  style: const TextStyle(
                    color: Colors.white,
                    fontSize: 10,
                    fontWeight: FontWeight.w500,
                  ),
                  overflow: TextOverflow.ellipsis,
                ),
              ),
            ),
          ],
        ),
      ),
    );
  }

  Widget _buildContent(BuildContext context, WidgetRef ref) {
    final state = ref.watch(karbeatStateProvider);

    switch (clip.source) {
      case UiClipSource_Audio(:final sourceId):
        double ratio = 1.0;

        final audioData = waveformMap[sourceId];
        if (audioData == null) {
          return const Center(
            child: Text("Loading...", style: TextStyle(fontSize: 8)),
          );
        }
        if (projectSampleRate > 0 && audioData.sampleRate > 0) {
          ratio = audioData.sampleRate / projectSampleRate;
        }

        final double effectiveOffset =
            overrideOffset ?? clip.offsetStart.toDouble();

        return RepaintBoundary(
          child: CustomPaint(
            size: Size.infinite, // Fill the clip container
            painter: StereoWaveformClipPainter(
              samples: audioData.previewBuffer,
              color: Colors.white.withAlpha(200),
              zoomLevel: zoomLevel,
              offsetSamples: effectiveOffset,
              strokeWidth: 1.0,
              ratio: ratio,
              scrollController: scrollController,
              clipLeftOffset: clipLeftOffset,
            ),
          ),
        );
      case UiClipSource_Midi(:final patternId):
        final pattern = state.patterns[patternId];

        if (pattern == null) {
          state.syncPatternList();
          return const Center(
            child: Text(
              "?",
              style: TextStyle(color: Colors.white54, fontSize: 10),
            ),
          );
        }

        return RepaintBoundary(
          child: CustomPaint(
            size: Size.infinite,
            painter: MidiClipPainter(
              pattern: pattern,
              color: color,
              zoomLevel: zoomLevel,
              sampleRate: projectSampleRate,
              bpm: state.transport.bpm,
              scrollController: scrollController,
              clipLeftOffset: clipLeftOffset,
            ),
          ),
        );
      default:
        return const SizedBox();
    }
  }
}

class _GridPainter extends CustomPainter {
  final double zoomLevel;
  final int gridSize;
  final double tempo;
  final int sampleRate;
  final ScrollController scrollController;

  _GridPainter({
    required this.zoomLevel,
    required this.gridSize,
    required this.tempo,
    required this.sampleRate,
    required this.scrollController,
  }) : super(repaint: scrollController);

  @override
  void paint(Canvas canvas, Size size) {
    if (tempo <= 0 || sampleRate <= 0 || zoomLevel <= 0 || gridSize <= 0) {
      return;
    }

    final double samplesPerBeat = (60.0 / tempo) * sampleRate;
    final double samplesPerGridLine = samplesPerBeat * (4.0 / gridSize);
    double pixelsPerGridLine = samplesPerGridLine / zoomLevel;

    if (pixelsPerGridLine < 0.0001) return;

    int skipFactor = 1;
    while (pixelsPerGridLine * skipFactor < 15.0) {
      skipFactor *= 2;
      if (skipFactor > 1000000) break;
    }

    final double visualInterval = pixelsPerGridLine * skipFactor;

    double startX = 0.0;
    double endX = size.width;

    if (scrollController.hasClients) {
      final position = scrollController.positions.first;
      final double offset = position.pixels;
      double viewportWidth = size.width;
      if (scrollController.position.hasViewportDimension) {
        viewportWidth = position.viewportDimension;
      }

      const double buffer = 200.0;
      startX = (offset - buffer).clamp(0.0, double.infinity);
      endX = offset + viewportWidth + buffer;
    }

    final paint = Paint()
      ..color = Colors.white.withAlpha((0.08 * 255).round())
      ..strokeWidth = 1.0;

    final barPaint = Paint()
      ..color = Colors.white.withAlpha((0.25 * 255).round())
      ..strokeWidth = 1.0;

    // Calculate start index
    int gridIndex = (startX / visualInterval).floor();

    // Use multiplication instead of addition to prevent float drift
    double currentX = gridIndex * visualInterval;

    while (currentX < endX) {
      if (currentX > size.width) break;

      int actualGridLines = gridIndex * skipFactor;
      // Is this line a Bar line? (Every 'gridSize' lines is a whole note/bar)
      bool isBar = (actualGridLines % gridSize == 0);

      if (currentX >= 0) {
        canvas.drawLine(
          Offset(currentX, 0),
          Offset(currentX, size.height),
          isBar ? barPaint : paint,
        );
      }

      // Increment index and recalculate X to stay precise
      gridIndex++;
      currentX = gridIndex * visualInterval;
    }
  }

  @override
  bool shouldRepaint(covariant _GridPainter oldDelegate) {
    return oldDelegate.zoomLevel != zoomLevel ||
        oldDelegate.gridSize != gridSize ||
        oldDelegate.tempo != tempo ||
        oldDelegate.sampleRate != sampleRate ||
        oldDelegate.scrollController != scrollController;
  }
}

final trackWaveformProvider =
    FutureProvider.family<Map<int, AudioWaveformUiForClip>, ({int trackId})>((
      ref,
      arg, // Access fields via the record variable
    ) async {
      final trackId = arg.trackId;

      ref.watch(karbeatStateProvider.select((s) => s.tracks[trackId]));

      final result = await getAudioWaveformForClipOnlyInSpecificTrack(
        trackId: trackId,
      );

      return result;
    });

int computeTargetBin(double zoomLevel) {
  if (zoomLevel <= 1) return 1;

  const levels = [1, 4, 16, 64, 256, 1024];

  for (final l in levels) {
    if (l >= zoomLevel) return l;
  }

  return levels.last; // fallback (max zoomed out)
}

/// Snaps a sample value to the nearest grid line based on the global state
int _snapTime(int samples, KarbeatState state) {
  if (!state.snapToGrid) return samples;

  final tempo = state.tempo;
  final sampleRate = state.hardwareConfig.sampleRate > 0
      ? state.hardwareConfig.sampleRate
      : 48000;
  final gridSize = state.gridSize;

  if (tempo <= 0 || sampleRate <= 0 || gridSize <= 0) return samples;

  // Calculate the exact sample width of one grid line
  final double samplesPerBeat = (60.0 / tempo) * sampleRate;
  final double samplesPerGridLine = samplesPerBeat * (4.0 / gridSize);

  if (samplesPerGridLine <= 0) return samples;

  // Round to the nearest grid interval
  return ((samples / samplesPerGridLine).round() * samplesPerGridLine).toInt();
}

class _GroupedBatchOverlay extends ConsumerWidget {
  final List<int> trackIds;
  final double headerWidth;
  final double itemHeight;
  final ScrollController horizontalScrollController;
  final ScrollController timelineController;
  final ClipDragController clipDragController;

  const _GroupedBatchOverlay({
    required this.trackIds,
    required this.headerWidth,
    required this.itemHeight,
    required this.horizontalScrollController,
    required this.timelineController,
    required this.clipDragController,
  });

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    return AnimatedBuilder(
      animation: clipDragController,
      builder: (context, child) {
        if (!clipDragController.isActive) return const SizedBox();

        final state = ref.read(karbeatStateProvider);
        final selectedClipIds = state.selectedClipIds;
        final selectedTrackId = state.selectedTrackId;

        if (selectedTrackId == null || selectedClipIds.isEmpty)
          return const SizedBox();

        final track = state.tracks[selectedTrackId];
        if (track == null) return const SizedBox();

        final zoomLevel = state.horizontalZoomLevel;
        double scrollX = horizontalScrollController.hasClients
            ? horizontalScrollController.offset
            : 0;
        double scrollY = timelineController.hasClients
            ? timelineController.offset
            : 0;

        final sortedTracks = trackIds..sort((a, b) => a.compareTo(b));
        final trackIndex = sortedTracks.indexWhere((t) => t == selectedTrackId);
        if (trackIndex < 0) return const SizedBox();

        // Pull the live math directly from the controller
        final snappedDelta = clipDragController.snappedDeltaSamples;
        final verticalDy = clipDragController.verticalDragDy;

        Widget groupedClips = Stack(
          clipBehavior: Clip.none,
          children: selectedClipIds.map((clipId) {
            final clip = track.clips.where((c) => c.id == clipId).firstOrNull;
            if (clip == null) return const SizedBox();

            final screenLeft =
                (clip.startTime / zoomLevel) - scrollX + headerWidth;
            final screenTop = (trackIndex * itemHeight) - scrollY + 30 + 2;
            final clipWidth = clip.loopLength / zoomLevel;

            double activeWidth = clipWidth;
            double activeLeft = screenLeft;

            if (clipDragController.action == BatchDragAction.resizeRight) {
              activeWidth += (snappedDelta / zoomLevel);
            } else if (clipDragController.action ==
                BatchDragAction.resizeLeft) {
              activeLeft += (snappedDelta / zoomLevel);
              activeWidth -= (snappedDelta / zoomLevel);
            }

            return Positioned(
              left: activeLeft,
              top: screenTop,
              width: activeWidth < 1 ? 1 : activeWidth,
              height: itemHeight - 4,
              child: Container(
                decoration: BoxDecoration(
                  color: Colors.cyanAccent.withAlpha(150),
                  border: Border.all(color: Colors.cyanAccent, width: 2),
                  borderRadius: BorderRadius.circular(4),
                ),
                child: Center(
                  child: Text(
                    clip.name,
                    style: const TextStyle(
                      color: Colors.black,
                      fontSize: 10,
                      fontWeight: FontWeight.bold,
                    ),
                    overflow: TextOverflow.ellipsis,
                  ),
                ),
              ),
            );
          }).toList(),
        );

        if (clipDragController.action == BatchDragAction.move) {
          groupedClips = Transform.translate(
            offset: Offset((snappedDelta / zoomLevel), verticalDy),
            child: groupedClips,
          );
        }

        return Positioned.fill(
          child: IgnorePointer(
            child: groupedClips,
          ),
        );
      },
    );
  }
}
