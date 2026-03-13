import 'dart:async';

import 'package:flutter/gestures.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:karbeat/features/playlist/clip_drag_controller.dart';
import 'package:karbeat/features/playlist/playhead.dart';
import 'package:karbeat/features/playlist/track_slot.dart';
import 'package:karbeat/features/components/context_menu.dart';
import 'package:karbeat/src/rust/api/plugin.dart' show UiPluginInfo;
import 'package:karbeat/src/rust/api/project.dart';
import 'package:karbeat/src/rust/core/project/track.dart';
import 'package:karbeat/state/app_state.dart';
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
            final tracks = ref.watch(
              karbeatStateProvider.select((s) => s.tracks),
            );
            final sortedTracks = tracks.values.toList()
              ..sort((a, b) => a.id.compareTo(b.id));

            return _SplitTrackView(
              tracks: sortedTracks,
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
  final List<UiTrack> tracks;
  final double itemHeight;
  final double headerWidth;

  const _SplitTrackView({
    required this.tracks,
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

    final zoomLevel = state.horizontalZoomLevel;

    switch (state.selectedTool) {
      case ToolSelection.scrub:
        final double samples = absoluteX * zoomLevel;
        state.seekTo(samples.toInt());
        break;
      case ToolSelection.zoom:
        break;
      case ToolSelection.draw:
        setState(() {
          _mousePos = localPosition;
        });
        _updatePlacementTarget(state);
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
    trackIndex = trackIndex.clamp(0, widget.tracks.length - 1);

    setState(() {
      _isRangeSelecting = true;
      _rangeSelectStart = Offset(absoluteX, absoluteY);
      _rangeSelectEnd = Offset(absoluteX, absoluteY);
      _rangeSelectTrackId = widget.tracks[trackIndex].id;
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

  @override
  Widget build(BuildContext context) {
    // Calculate total height to ensure both lists have exactly same extent
    // +1 for the Add Button row
    final int itemCount = widget.tracks.length + 1;
    final state = ref.read(karbeatStateProvider);
    final isPlacing = ref.watch(
      karbeatStateProvider.select((s) => s.isPlacing),
    );
    final selectedTool = ref.watch(
      karbeatStateProvider.select((s) => s.selectedTool),
    );
    final horizontalZoom = ref.watch(
      karbeatStateProvider.select((s) => s.horizontalZoomLevel),
    );
    final currentTimelineWidth = _timelineWidth;

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
                        if (index == widget.tracks.length) {
                          return _buildAddButton();
                        }
                        return _buildTrackHeader(widget.tracks[index]);
                      },
                    ),
                  ),
                ],
              ),
            ),

            Container(width: 1, color: Colors.black),

            // ============ RIGHT: TIMELINE ==============
            Expanded(
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
                    if (selectedTool == ToolSelection.scrub) {
                      _handleTimelineGesture(
                        context,
                        details.localPosition,
                        isDrag: true,
                      );
                      return;
                    }
                    if (selectedTool == ToolSelection.draw || isPlacing) {
                      setState(() => _mousePos = details.localPosition);
                      _updatePlacementTarget(state);
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
                          double absoluteX = details.localPosition.dx + scrollX;
                          final samples = absoluteX * state.horizontalZoomLevel;
                          state.seekTo(samples.toInt());
                        },
                        onPanUpdate: (details) {
                          double scrollX = _rulerController.hasClients
                              ? _rulerController.offset
                              : 0;
                          double absoluteX = details.localPosition.dx + scrollX;
                          final samples = absoluteX * state.horizontalZoomLevel;
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
                          cursor: isPlacing
                              ? SystemMouseCursors.move
                              : selectedTool == ToolSelection.select
                              ? SystemMouseCursors.precise
                              : SystemMouseCursors.basic,
                          onHover: null,
                          child: GestureDetector(
                            behavior: HitTestBehavior.translucent,
                            onPanUpdate: null,
                            onTapDown: isPlacing
                                ? (details) {
                                    setState(() {
                                      _mousePos = details.localPosition;
                                    });
                                    _updatePlacementTarget(state);
                                  }
                                : null,
                            child: ScrollConfiguration(
                              // Only allow Mouse Drag scrolling when using Pointer or Move tool
                              behavior:
                                  (selectedTool == ToolSelection.pointer ||
                                      selectedTool == ToolSelection.move)
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
                                        if (index == widget.tracks.length) {
                                          return SizedBox(height: 60);
                                        }
                                        return IgnorePointer(
                                          ignoring: isPlacing,
                                          child: KarbeatTrackSlot(
                                            trackId: widget.tracks[index].id,
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
          ],
        ),
        if (isPlacing && _mousePos != null) _buildGhostClip(context),
        if (_isRangeSelecting) _buildRangeSelectRect(context),
        // Batch drag overlays for all selected clips during move
        ..._buildBatchDragOverlays(context),
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
                  onPressed: () => state.cancelPlacement(),
                ),
                const SizedBox(width: 16),
                FloatingActionButton.extended(
                  onPressed: () {
                    final messenger = ScaffoldMessenger.of(context);
                    state.confirmPlacement().then((value) {
                      if (!mounted) return;
                      switch (value) {
                        case Ok<void>():
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

  void _updatePlacementTarget(KarbeatState state) {
    if (_mousePos == null) return;

    // Calculate Absolute Y (Mouse + Scroll)
    double scrollY = 0;
    if (_timelineController.hasClients) {
      scrollY = _timelineController.offset;
    }
    double absoluteY = _mousePos!.dy + scrollY;

    // Determine Track Index
    int trackIndex = (absoluteY / widget.itemHeight).floor();
    trackIndex = trackIndex.clamp(0, widget.tracks.length - 1);
    final targetTrack = widget.tracks[trackIndex];

    // Calculate Absolute X (Mouse + Scroll)
    double scrollX = 0;
    if (_trackContentController.hasClients) {
      scrollX = _trackContentController.offset;
    }
    double absoluteX = _mousePos!.dx + scrollX;

    if (absoluteX < 0) absoluteX = 0;

    // Convert X Pixels -> Samples
    final zoomLevel = ref.read(karbeatStateProvider).horizontalZoomLevel;
    double samples = absoluteX * zoomLevel;

    state.updatePlacementTarget(targetTrack.id, samples);
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
    trackIndex = trackIndex.clamp(0, widget.tracks.length - 1);

    double top = (trackIndex * widget.itemHeight) - scrollY;

    // Offset by header height (approx) + Header Row
    top += 30;

    // Left position is just mouse X offset by header width
    double left = widget.headerWidth + _mousePos!.dx;

    // Safety check to keep it in timeline area
    if (left < widget.headerWidth) left = widget.headerWidth;

    return Positioned(
      left: left,
      top: top,
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
          final state = ref.read(karbeatStateProvider);
          _updatePlacementTarget(state);
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
    final trackIndex = widget.tracks.indexWhere(
      (t) => t.id == _rangeSelectTrackId,
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

  /// Builds overlay widgets for all selected clips during a batch move operation
  List<Widget> _buildBatchDragOverlays(BuildContext context) {
    // Only show during batch move
    if (_clipDragController.action != BatchDragAction.move) {
      return [];
    }

    final state = ref.read(karbeatStateProvider);
    final selectedClipIds = state.selectedClipIds;
    final selectedTrackId = state.selectedTrackId;
    if (selectedTrackId == null || selectedClipIds.isEmpty) return [];

    // Get the track and its clips
    final track = state.tracks[selectedTrackId];
    if (track == null) return [];

    // Calculate scroll offsets
    double scrollX = 0;
    double scrollY = 0;
    if (_trackContentController.hasClients) {
      scrollX = _trackContentController.offset;
    }
    if (_timelineController.hasClients) {
      scrollY = _timelineController.offset;
    }

    // Find source track index
    final sortedTracks = widget.tracks..sort((a, b) => a.id.compareTo(b.id));
    final trackIndex = sortedTracks.indexWhere((t) => t.id == selectedTrackId);
    if (trackIndex < 0) return [];

    // Calculate target track based on vertical delta
    final rowOffset = _clipDragController.deltaRows.round();
    final targetTrackIndex = (trackIndex + rowOffset).clamp(
      0,
      sortedTracks.length - 1,
    );

    final zoomLevel = state.horizontalZoomLevel;
    final deltaSamples = _clipDragController.deltaSamples;

    final List<Widget> overlays = [];

    for (final clipId in selectedClipIds) {
      final clip = track.clips.where((c) => c.id == clipId).firstOrNull;
      if (clip == null) continue;

      // Calculate new position with delta applied
      final newStartTime = (clip.startTime + deltaSamples).clamp(
        0,
        double.maxFinite.toInt(),
      );
      final screenLeft =
          (newStartTime / zoomLevel) - scrollX + widget.headerWidth;
      final screenTop =
          (targetTrackIndex * widget.itemHeight) - scrollY + 30 + 2;
      final clipWidth = clip.loopLength / zoomLevel;

      overlays.add(
        Positioned(
          left: screenLeft,
          top: screenTop,
          width: clipWidth < 1 ? 1 : clipWidth,
          height: widget.itemHeight - 4,
          child: IgnorePointer(
            child: Opacity(
              opacity: 0.7,
              child: Container(
                decoration: BoxDecoration(
                  color: Colors.cyanAccent.withAlpha(100),
                  border: Border.all(color: Colors.cyanAccent, width: 2),
                  borderRadius: BorderRadius.circular(4),
                ),
                child: Center(
                  child: Text(
                    clip.name,
                    style: const TextStyle(
                      color: Colors.white,
                      fontSize: 10,
                      fontWeight: FontWeight.bold,
                      shadows: [Shadow(color: Colors.black, blurRadius: 2)],
                    ),
                    overflow: TextOverflow.ellipsis,
                  ),
                ),
              ),
            ),
          ),
        ),
      );
    }

    return overlays;
  }

  final Map<Color, Color> _contrastColorCache = {};

  // 2. Create a helper function that checks the cache first
  Color _getContrastColor(Color backgroundColor) {
    // putIfAbsent checks if the key exists. If it does, it returns the cached value instantly.
    // If it doesn't, it runs the expensive computeLuminance() exactly once, caches it, and returns it.
    return _contrastColorCache.putIfAbsent(backgroundColor, () {
      return backgroundColor.computeLuminance() > 0.5
          ? Colors.black
          : Colors.white;
    });
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

  Widget _buildTrackHeader(UiTrack track) {
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
        height: widget.itemHeight,
        child: Container(
          margin: const EdgeInsets.only(bottom: 2),
          padding: const EdgeInsets.symmetric(horizontal: 10),
          decoration: BoxDecoration(
            color: Color(int.parse(track.color.substring(1), radix: 16)),
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
                        color: _getContrastColor(
                          Color(int.parse(track.color.substring(1), radix: 16)),
                        ),
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

  IconData _getTrackIcon(TrackType type) {
    switch (type) {
      case TrackType.audio:
        return Icons.graphic_eq;
      case TrackType.midi:
        return Icons.piano;
      case TrackType.automation:
        return Icons.show_chart;
    }
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
              ref.read(karbeatStateProvider).addTrack(TrackType.audio);
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
