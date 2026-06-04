import 'dart:async';
import 'package:karbeat/features/components/scroll_physics/unclamped_never_scrollable_physics.dart';
import 'package:karbeat/models/grid.dart';
import 'dart:math' as math;

import 'package:flutter/gestures.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:karbeat/features/components/context_menu.dart';
import 'package:karbeat/features/components/midi_drawer.dart';
import 'package:karbeat/features/components/waveform_painter.dart';
import 'package:karbeat/features/playlist/clip_drag_controller.dart';
import 'package:karbeat/features/playlist/playhead.dart';
import 'package:karbeat/models/id.dart';
import 'package:karbeat/models/interaction_target.dart';
import 'package:karbeat/src/rust/api/automation.dart';
import 'package:karbeat/src/rust/api/plugin.dart' show UiPluginInfo;
import 'package:karbeat/src/rust/api/project.dart';
import 'package:karbeat/src/rust/api/track.dart';
import 'package:karbeat/src/rust/api/waveform.dart';
import 'package:karbeat/state/app_state.dart';
import 'package:karbeat/state/clip_placement_state.dart';
import 'package:karbeat/utils/clip_time_utils.dart';
import 'package:karbeat/utils/color.dart';
import 'package:karbeat/utils/logger.dart';
import 'package:karbeat/utils/math.dart';
import 'package:karbeat/utils/result_type.dart';
import 'package:karbeat/utils/scroll_behavior.dart';
import 'package:linked_scroll_controller/linked_scroll_controller.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:multi_split_view/multi_split_view.dart';

class TrackListScreen extends ConsumerWidget {
  const TrackListScreen({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    return LayoutBuilder(
      builder: (context, constraints) {
        final parentHeight = constraints.maxHeight;
        final parentWidth = constraints.maxWidth;
        if (parentHeight.isInfinite) return const SizedBox();

        final calculatedHeight = parentHeight * 0.15;
        final double itemHeight = calculatedHeight.clamp(60.0, 150.0);
        final double headerWidth = parentWidth > 600
            ? 220.0
            : parentWidth * 0.35;

        return Builder(
          builder: (context) {
            final trackIdsStr = ref.watch(
              globalStateProvider.select((s) {
                // Sort the actual track objects by their orderIdx
                final sortedTracks = s.tracks.values.toList()
                  ..sort((a, b) => a.orderIdx.compareTo(b.orderIdx));

                // Map them back to just their IDs
                return sortedTracks.map((t) => t.id).join(',');
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

class _TrackContainer extends ConsumerStatefulWidget {
  @override
  _TrackContainerState createState() {
    return _TrackContainerState();
  }
}

class _TrackContainerState extends ConsumerState<_TrackContainer> {
  @override
  Widget build(BuildContext context) {
    // TODO: implement build
    throw UnimplementedError();
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

  late MultiSplitViewController _trackSplitViewController;

  // Local state for ghost clip
  Offset? _mousePos;
  Offset? _lastRightClickPos;

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
    _trackSplitViewController = MultiSplitViewController(
      areas: [
        Area(size: widget.headerWidth, min: 80, max: 240, data: 'header'),
        Area(min: 200, data: 'timeline'),
      ],
    );

    // Initialize the Linked Group
    _verticalControllers = LinkedScrollControllerGroup();
    _headerController = _verticalControllers.addAndGet();
    _timelineController = _verticalControllers.addAndGet();
    _horizontalControllers = LinkedScrollControllerGroup();
    _rulerController = _horizontalControllers.addAndGet();
    _trackContentController = _horizontalControllers.addAndGet();
    _trackContentController.addListener(_handleScrollExpansion);
    HardwareKeyboard.instance.addHandler(_handleKeyEvents);

    final state = ref.read(globalStateProvider);
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
    _trackSplitViewController.dispose();
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
    final state = ref.read(globalStateProvider);
    final oldZoom = state.horizontalZoomLevel;

    final clampedZoom = newZoom.clamp(1.0, 1000.0);
    if (clampedZoom == oldZoom) return;

    final controller = _trackContentController;
    double currentScroll = controller.hasClients ? controller.offset : 0.0;
    double viewportWidth = controller.hasClients
        ? controller.position.viewportDimension
        : 1000.0;

    // Pivot Math: Lock the exact musical time under the cursor
    final double ticksAtFocalPoint = (currentScroll + focalPointX) * oldZoom;
    double newScroll = (ticksAtFocalPoint / clampedZoom) - focalPointX;
    if (newScroll < 0) newScroll = 0.0;

    // Dynamic Window Strategy
    // Find the actual furthest tick of content in the project
    double maxContentTicks = 3840.0; // Base minimum 1 bar
    for (final track in state.tracks.values) {
      for (final clip in track.clips) {
        final endTick =
            clip.startTimeInTicks(state.tempo, _activeSampleRate) +
            clip.loopLengthInTicks(state.tempo, _activeSampleRate);
        if (endTick > maxContentTicks) {
          maxContentTicks = endTick.toDouble();
        }
      }
    }
    // Add 1 bar of padding to the end of the song
    maxContentTicks += 3840.0;

    // The pixel width required for the actual song content
    double contentWidth = maxContentTicks / clampedZoom;

    // The pixel width required to support the new scroll position + viewport size
    double requiredWindowWidth = newScroll + viewportWidth;

    // Set the timeline to wrap whichever is larger.
    // This allows zooming infinitely without clamping, but cleanly shrinks
    // the scrollbar exactly to the edge of the content when zooming out.
    double newTimelineWidth = math.max(contentWidth, requiredWindowWidth);

    // Update the state immediately
    state.horizontalZoomLevel = clampedZoom;
    setState(() {
      _timelineWidth = newTimelineWidth;
    });

    // Synchronous Jump
    // Because we are using UnclampedNeverScrollableScrollPhysics, this jump will
    // bypass Flutter's 1-frame layout boundary check, guaranteeing a flawless pivot.
    if (controller.hasClients) {
      controller.jumpTo(newScroll);
    }
  }

  void _handleTimelineGesture(BuildContext context, Offset localPosition) {
    final state = ref.read(globalStateProvider);
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
      case ToolSelection.slice:
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
  void _confirmRangeSelect(GlobalAppState state) {
    if (!_isRangeSelecting ||
        _rangeSelectStart == null ||
        _rangeSelectEnd == null ||
        _rangeSelectTrackId == null) {
      _cancelRangeSelect();
      return;
    }

    final zoomLevel = state.horizontalZoomLevel;

    // Get time range in ticks
    final startX = _rangeSelectStart!.dx;
    final endX = _rangeSelectEnd!.dx;
    final minX = startX < endX ? startX : endX;
    final maxX = startX > endX ? startX : endX;

    final startTimeTicks = (minX * zoomLevel).toInt();
    final endTimeTicks = (maxX * zoomLevel).toInt();

    final bpm = state.tempo;
    final sr = state.hardwareConfig.sampleRate;

    // Find clips in the target track that overlap with the selection range
    final track = state.tracks[_rangeSelectTrackId!];
    if (track == null) {
      _cancelRangeSelect();
      return;
    }

    final selectedClipIds = <int>[];
    for (final clip in track.clips) {
      final clipStart = clip.startTimeInTicks(bpm, sr);
      final clipEnd = clipStart + clip.loopLengthInTicks(bpm, sr);

      // Check if clip overlaps with selection range
      if (clipEnd > startTimeTicks && clipStart < endTimeTicks) {
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
  Widget _buildCutHelperLine(BuildContext context, GlobalAppState state) {
    if (_mousePos == null || state.selectedTool != ToolSelection.slice) {
      return const SizedBox();
    }

    double scrollX = 0;
    if (_trackContentController.hasClients) {
      scrollX = _trackContentController.offset;
    }

    double absoluteX = _mousePos!.dx + scrollX;
    if (absoluteX < 0) absoluteX = 0;

    final zoomLevel = state.horizontalZoomLevel;
    double ticks = absoluteX * zoomLevel;

    // Apply Shift Step Snapping for cut
    ticks = _snapClipShiftTick(ticks.toInt(), state).toDouble();

    double snappedAbsoluteX = ticks / zoomLevel;
    double left = snappedAbsoluteX - scrollX;

    // Hide if scrolled out of view to the left
    if (left < 0) return const SizedBox();

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

  Widget _buildToolbar() {
    return Container(
      height: 36,
      color: Colors.grey.shade900,
      padding: const EdgeInsets.symmetric(horizontal: 16),
      child: SingleChildScrollView(
        scrollDirection: Axis.horizontal,
        child: Row(
          children: [
            const Text(
              "Snap to Grid",
              style: TextStyle(color: Colors.white70, fontSize: 12),
            ),
            const SizedBox(width: 8),
            DropdownButton<GridSize>(
              value: ref.watch(globalStateProvider.select((s) => s.gridSize)),
              dropdownColor: Colors.grey.shade800,
              style: const TextStyle(color: Colors.white, fontSize: 12),
              underline: const SizedBox(),
              items: GridSize.values.map((size) {
                String label = "";
                switch (size) {
                  case GridSize.full:
                    label = "1/1";
                    break;
                  case GridSize.half:
                    label = "1/2";
                    break;
                  case GridSize.third:
                    label = "1/3";
                    break;
                  case GridSize.quarter:
                    label = "1/4";
                    break;
                  case GridSize.sixth:
                    label = "1/6";
                    break;
                  case GridSize.eighth:
                    label = "1/8";
                    break;
                  case GridSize.sixteenth:
                    label = "1/16";
                    break;
                  case GridSize.thirtysecond:
                    label = "1/32";
                    break;
                  case GridSize.sixtyfourth:
                    label = "1/64";
                    break;
                  case GridSize.oneBar:
                    label = "1 Bar";
                    break;
                  case GridSize.twoBeat:
                    label = "2 Beats";
                    break;
                  case GridSize.infinity:
                    label = "None";
                    break;
                  case GridSize.twelfth:
                    label = "1/12";
                    break;
                }
                return DropdownMenuItem<GridSize>(
                  value: size,
                  child: Text(label),
                );
              }).toList(),
              onChanged: (val) {
                if (val != null) ref.read(globalStateProvider).setGridSize(val);
              },
            ),
            const SizedBox(width: 16),
            const Text(
              "Move Step",
              style: TextStyle(color: Colors.white70, fontSize: 12),
            ),
            const SizedBox(width: 8),
            DropdownButton<MusicalBeatSize>(
              value: ref.watch(
                globalStateProvider.select(
                  (s) => s.horizontalClipShiftSizeDenom,
                ),
              ),
              dropdownColor: Colors.grey.shade800,
              style: const TextStyle(color: Colors.white, fontSize: 12),
              underline: const SizedBox(),
              items: MusicalBeatSize.values.map((size) {
                String label = "";
                switch (size) {
                  case MusicalBeatSize.four:
                    label = "1 Bar";
                    break;
                  case MusicalBeatSize.three:
                    label = "3 Beats";
                    break;
                  case MusicalBeatSize.two:
                    label = "2 Beats";
                    break;
                  case MusicalBeatSize.one:
                    label = "1 Beat";
                    break;
                  case MusicalBeatSize.half:
                    label = "1/2 Step";
                    break;
                  case MusicalBeatSize.quarter:
                    label = "1/4 Step";
                    break;
                  case MusicalBeatSize.eighth:
                    label = "1/8 Step";
                    break;
                  case MusicalBeatSize.sixteenth:
                    label = "1/16 Step";
                    break;
                  case MusicalBeatSize.thirtysecond:
                    label = "1/32 Step";
                    break;
                  case MusicalBeatSize.sixtyfourth:
                    label = "1/64 Step";
                    break;
                  case MusicalBeatSize.none:
                    label = "None";
                    break;
                  case MusicalBeatSize.third:
                    label = "1/3 Step";
                    break;
                  case MusicalBeatSize.sixth:
                    label = "1/6 Step";
                    break;
                  case MusicalBeatSize.twelfth:
                    label = "1/12 Step";
                    break;
                }
                return DropdownMenuItem<MusicalBeatSize>(
                  value: size,
                  child: Text(label),
                );
              }).toList(),
              onChanged: (val) {
                if (val != null) {
                  ref.read(globalStateProvider).horizontalClipShiftSizeDenom =
                      val;
                }
              },
            ),
          ],
        ),
      ),
    );
  }

  Widget _buildHeaderArea(int itemCount) {
    return Column(
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
            controller: _headerController,
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
    );
  }

  Widget _buildTimelineArea(BuildContext context, int itemCount) {
    final state = ref.read(globalStateProvider);
    final isPlacing = ref.watch(
      clipPlacementProvider.select((s) => s.isPlacing),
    );
    final selectedTool = ref.watch(
      globalStateProvider.select((s) => s.selectedTool),
    );
    final horizontalZoom = ref.watch(
      globalStateProvider.select((s) => s.horizontalZoomLevel),
    );
    final selectedClipIds = ref.watch(
      globalStateProvider.select((s) => s.selectedClipIds),
    );
    final bool isZooming = _isCtrlPressed || selectedTool == ToolSelection.zoom;

    MouseCursor handleCursor() {
      if (isPlacing) return SystemMouseCursors.move;
      if (selectedTool == ToolSelection.select) {
        return SystemMouseCursors.precise;
      }
      if (selectedTool == ToolSelection.slice) return SystemMouseCursors.text;
      return SystemMouseCursors.basic;
    }

    return Stack(
      children: [
        Column(
          children: [
            GestureDetector(
              onTapDown: (details) {
                double scrollX = _rulerController.hasClients
                    ? _rulerController.offset
                    : 0;
                double absoluteX = details.localPosition.dx + scrollX;
                final ticks = absoluteX * state.horizontalZoomLevel;
                final sampleRate = _activeSampleRate > 0
                    ? _activeSampleRate
                    : 48000;
                final samples =
                    (ticks * (60.0 / state.tempo) * (sampleRate / 960.0))
                        .round();
                state.seekTo(samples);
              },
              onPanUpdate: (details) {
                double scrollX = _rulerController.hasClients
                    ? _rulerController.offset
                    : 0;
                double absoluteX = details.localPosition.dx + scrollX;
                final ticks = absoluteX * state.horizontalZoomLevel;
                final sampleRate = _activeSampleRate > 0
                    ? _activeSampleRate
                    : 48000;
                final samples =
                    (ticks * (60.0 / state.tempo) * (sampleRate / 960.0))
                        .round();
                state.seekTo(samples);
              },
              child: Container(
                height: 30,
                color: Colors.grey.shade800,
                width: double.infinity,
                child: SingleChildScrollView(
                  scrollDirection: Axis.horizontal,
                  controller: _rulerController,
                  physics: isZooming
                      ? const UnclampedNeverScrollableScrollPhysics()
                      : const ClampingScrollPhysics(),
                  child: SizedBox(
                    width: _timelineWidth,
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
                onHover: (event) {
                  _lastRightClickPos = event.localPosition;
                  if (selectedTool == ToolSelection.slice ||
                      selectedTool == ToolSelection.draw) {
                    setState(() => _mousePos = event.localPosition);
                  }
                },
                onExit: (_) {
                  if (isPlacing) return;
                  if (_mousePos != null) setState(() => _mousePos = null);
                },
                child: Listener(
                  onPointerDown: (event) {
                    _lastRightClickPos = event.localPosition;
                    _mousePos = event.localPosition;
                    if (event.buttons == kSecondaryButton) {
                      _lastRightClickPos = event.localPosition;
                      ref.read(globalStateProvider).deselectAllClips();
                      setState(() {});
                    }
                  },
                  onPointerSignal: (event) {
                    if (event is PointerScrollEvent && _isCtrlPressed) {
                      final currentZoom = ref
                          .read(globalStateProvider)
                          .horizontalZoomLevel;
                      final double multiplier = event.scrollDelta.dy > 0
                          ? 0.9
                          : 1.1;
                      _updateZoom(
                        currentZoom * multiplier,
                        event.localPosition.dx,
                      );
                    }
                  },
                  child: GestureDetector(
                    behavior: HitTestBehavior.translucent,
                    onPanUpdate: (details) {
                      if (selectedTool == ToolSelection.select) {
                        _updateRangeSelect(details.localPosition);
                        return;
                      }
                      if (selectedTool == ToolSelection.zoom) {
                        final currentZoom = ref
                            .read(globalStateProvider)
                            .horizontalZoomLevel;
                        final double multiplier = details.delta.dy > 0
                            ? 0.9
                            : 1.1;
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
                    onTapDown: isPlacing
                        ? (details) {
                            setState(() => _mousePos = details.localPosition);
                            _updatePlacementTarget();
                          }
                        : (details) => _handleTimelineGesture(
                            context,
                            details.localPosition,
                          ),
                    onPanStart: (details) {
                      if (selectedTool == ToolSelection.select) {
                        _startRangeSelect(details.localPosition);
                      }
                    },
                    onPanEnd: (details) {
                      if (selectedTool == ToolSelection.select &&
                          _isRangeSelecting) {
                        _confirmRangeSelect(state);
                      }
                    },
                    child: ScrollConfiguration(
                      behavior: (selectedTool == ToolSelection.pointer)
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
                          physics: isZooming
                              ? const UnclampedNeverScrollableScrollPhysics()
                              : const ClampingScrollPhysics(),
                          child: SizedBox(
                            width: _timelineWidth,
                            child: ContextMenuWrapper(
                              title: "Track Options",
                              actions: [
                                DawContextAction(
                                  title: "Paste",
                                  icon: Icons.paste,
                                  onTap: () async {
                                    final targetPos = _lastRightClickPos;
                                    if (targetPos == null) return;

                                    final currentState = ref.read(
                                      globalStateProvider,
                                    );
                                    double scrollY =
                                        _timelineController.hasClients
                                        ? _timelineController.offset
                                        : 0;
                                    double absoluteY = targetPos.dy + scrollY;
                                    int trackIndex =
                                        (absoluteY / widget.itemHeight).floor();
                                    trackIndex = trackIndex.clamp(
                                      0,
                                      widget.trackIds.length - 1,
                                    );

                                    final targetTrackId =
                                        widget.trackIds[trackIndex];
                                    final targetTrack =
                                        currentState.tracks[targetTrackId];
                                    if (targetTrack == null) return;

                                    double scrollX =
                                        _trackContentController.hasClients
                                        ? _trackContentController.offset
                                        : 0;
                                    double absoluteX = (targetPos.dx + scrollX)
                                        .clamp(0, double.infinity);
                                    double ticks =
                                        absoluteX *
                                        currentState.horizontalZoomLevel;

                                    if (currentState.snapToGrid) {
                                      ticks = _snapTick(
                                        ticks.toInt(),
                                        currentState,
                                      ).toDouble();
                                    }

                                    int pasteStartTime;
                                    if (targetTrack.trackType ==
                                        UiTrackType.audio) {
                                      final sr =
                                          currentState
                                                  .hardwareConfig
                                                  .sampleRate >
                                              0
                                          ? currentState
                                                .hardwareConfig
                                                .sampleRate
                                          : 48000;
                                      pasteStartTime = ticksToSamples(
                                        ticks.toInt(),
                                        currentState.tempo,
                                        sr,
                                      );
                                    } else {
                                      pasteStartTime = ticks.toInt();
                                    }

                                    final result = await currentState
                                        .pasteClips(
                                          targetTrackId: targetTrackId,
                                          pasteStartTime: pasteStartTime,
                                          trackType: targetTrack.trackType,
                                        );

                                    if (result.isErr() && context.mounted) {
                                      ScaffoldMessenger.of(
                                        context,
                                      ).showSnackBar(
                                        SnackBar(
                                          content: Text(
                                            (result as Error<void>)
                                                .toErrorMessage(),
                                          ),
                                        ),
                                      );
                                    } else if (result.isOk()) {
                                      AppLogger.info("Paste clip");
                                      setState(() => _lastRightClickPos = null);
                                    }
                                  },
                                ),
                              ],
                              child: ListView.builder(
                                controller: _timelineController,
                                physics: isZooming
                                    ? const UnclampedNeverScrollableScrollPhysics()
                                    : const ClampingScrollPhysics(),
                                padding: EdgeInsets.zero,
                                itemCount: itemCount,
                                itemBuilder: (context, index) {
                                  if (index == widget.trackIds.length) {
                                    return const SizedBox(height: 60);
                                  }
                                  return IgnorePointer(
                                    ignoring: isPlacing,
                                    child: AudioTrackSlot(
                                      trackId: widget.trackIds[index],
                                      height: widget.itemHeight,
                                      horizontalScrollController:
                                          _trackContentController,
                                      sampleRate: _activeSampleRate,
                                      clipDragController: _clipDragController,
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
              ),
            ),
          ],
        ),

        // Overlays inside the Timeline Stack
        if (isPlacing && _mousePos != null) _buildGhostClip(context),
        if (_isRangeSelecting) _buildRangeSelectRect(context),
        _buildCutHelperLine(context, state),

        _GroupedBatchOverlay(
          trackIds: widget.trackIds,
          itemHeight: widget.itemHeight,
          horizontalScrollController: _trackContentController,
          timelineController: _timelineController,
          clipDragController: _clipDragController,
        ),

        Positioned.fill(
          child: IgnorePointer(
            ignoring: false,
            child: PlayheadOverlay(
              offsetAdjustment: 0.0,
              scrollController: _trackContentController,
              zoomLevel: horizontalZoom,
              sampleSelector: (pos) => pos.ticks,
              onSeek: (int newTicks) {
                final state = ref.read(globalStateProvider);
                final tempo = state.tempo;
                final safeTicks = newTicks < 0 ? 0 : newTicks;
                final sampleRate = state.hardwareConfig.sampleRate > 0
                    ? state.hardwareConfig.sampleRate
                    : 48000;
                final samples =
                    (safeTicks * (60.0 / tempo) * (sampleRate / 960.0)).round();
                state.seekTo(samples);
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
                  onPressed: () async {
                    final result = await ref
                        .read(clipPlacementProvider.notifier)
                        .confirmPlacement();
                    if (!context.mounted) return;
                    switch (result) {
                      case Ok<void>():
                        if (!ref.read(clipPlacementProvider).isPlacing) {
                          setState(() => _mousePos = null);
                        }
                      case Error<void>():
                        ScaffoldMessenger.of(context).showSnackBar(
                          SnackBar(content: Text(result.toErrorMessage())),
                        );
                    }
                  },
                  label: const Text('Confirm'),
                  heroTag: 'confirm_place',
                  icon: const Icon(Icons.check),
                  backgroundColor: Colors.greenAccent,
                ),
              ],
            ),
          ),

        if (selectedClipIds.isNotEmpty)
          FloatingContextPanel(
            actions: [
              DawContextAction(
                title: "Copy",
                icon: Icons.copy,
                onTap: () async {
                  final trackId = state.selectedTrackId;
                  if (trackId == null) return;
                  final result = await state.copySelectedClips(
                    trackId: trackId,
                    clipIds: selectedClipIds,
                  );
                  if (!context.mounted) return;
                  if (result.isErr()) {
                    ScaffoldMessenger.of(context).showSnackBar(
                      SnackBar(
                        content: Text((result as Error<void>).toErrorMessage()),
                      ),
                    );
                  }
                },
              ),
              DawContextAction(
                title: "Cut",
                icon: Icons.cut,
                onTap: () async {
                  final trackId = state.selectedTrackId;
                  if (trackId == null) return;
                  final result = await state.cutSelectedClips(
                    trackId: trackId,
                    clipIds: selectedClipIds,
                  );
                  state.deselectAllClips();
                  if (!context.mounted) return;
                  if (result.isErr()) {
                    ScaffoldMessenger.of(context).showSnackBar(
                      SnackBar(
                        content: Text((result as Error<void>).toErrorMessage()),
                      ),
                    );
                  }
                },
              ),
              DawContextAction(
                title: "Delete",
                icon: Icons.delete,
                isDestructive: true,
                onTap: () async {
                  state.deleteSelectedClips();
                  state.deselectAllClips();
                },
              ),
            ],
            onClose: () => state.deselectAllClips(),
            title:
                "${selectedClipIds.length} Clip${selectedClipIds.length == 1 ? '' : 's'}",
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

    // Convert X Pixels -> Ticks
    final state = ref.read(globalStateProvider);
    final zoomLevel = state.horizontalZoomLevel;
    double ticks = absoluteX * zoomLevel;

    if (state.snapToGrid) {
      ticks = _snapTick(ticks.toInt(), state).toDouble();
    }
    ref
        .read(clipPlacementProvider.notifier)
        .updatePlacementTarget(targetTrack, ticks);
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

    final state = ref.read(globalStateProvider);
    double absoluteX = _mousePos!.dx + scrollX;
    if (absoluteX < 0) absoluteX = 0;

    double ticks = absoluteX * state.horizontalZoomLevel;
    if (state.snapToGrid) {
      ticks = _snapTick(ticks.toInt(), state).toDouble();
    }

    // Convert the snapped position back into a screen pixel coordinate
    double snappedAbsoluteX = ticks / state.horizontalZoomLevel;
    double left = math.max(snappedAbsoluteX - scrollX, 0);

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
    final screenLeft = minX - scrollX;
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
              ref.read(globalStateProvider).addAudioTrack();
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
    final availablePlugins = ref.read(globalStateProvider).availableGenerators;

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
        ref.read(globalStateProvider).addMidiTrackWithGeneratorId(plugin.id);
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

  Widget _buildBusAutomationLaneSection(BuildContext context, WidgetRef ref) {
    return Expanded(child: Container());
  }

  @override
  Widget build(BuildContext context) {
    // Calculate total height to ensure both lists have exactly same extent
    // +1 for the Add Button row
    final int itemCount = widget.trackIds.length + 1;
    return Column(
      children: [
        _buildToolbar(),
        Expanded(
          child: MultiSplitViewTheme(
            data: MultiSplitViewThemeData(
              dividerPainter: DividerPainters.grooved1(
                color: Colors.grey.shade800,
                highlightedColor: Colors.cyanAccent,
                thickness: 1,
              ),
            ),
            child: MultiSplitView(
              controller: _trackSplitViewController,
              builder: (context, area) {
                switch (area.data) {
                  case 'header':
                    return _buildHeaderArea(itemCount);
                  case 'timeline':
                    return _buildTimelineArea(context, itemCount);
                  default:
                    return const SizedBox();
                }
              },
            ),
          ),
        ),
      ],
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

  Future<Color?> _showColorPickerDialog(
    BuildContext context,
    Color currentColor,
  ) {
    return showDialog<Color>(
      context: context,
      builder: (ctx) {
        return AlertDialog(
          title: const Text("Select Track Color"),
          content: SingleChildScrollView(
            child: Wrap(
              spacing: 12.0,
              runSpacing: 12.0,
              children: dawColors.map((color) {
                final isSelected = currentColor.toARGB32() == color.toARGB32();
                return GestureDetector(
                  onTap: () => Navigator.of(ctx).pop(color),
                  child: Container(
                    width: 40,
                    height: 40,
                    decoration: BoxDecoration(
                      color: color,
                      shape: BoxShape.circle,
                      border: Border.all(
                        color: isSelected ? Colors.white : Colors.transparent,
                        width: isSelected ? 3 : 0,
                      ),
                      boxShadow: [
                        if (isSelected)
                          BoxShadow(
                            color: color.withAlpha(100),
                            blurRadius: 8,
                            spreadRadius: 2,
                          ),
                      ],
                    ),
                  ),
                );
              }).toList(),
            ),
          ),
          actions: [
            TextButton(
              onPressed: () => Navigator.of(ctx).pop(),
              child: const Text("Cancel"),
            ),
          ],
        );
      },
    );
  }

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    // Only rebuilds this specific header if the track's name/color/type changes
    final track = ref.watch(
      globalStateProvider.select((s) => s.tracks[trackId]),
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
                  color: track.color.toColor(),
                  borderRadius: BorderRadius.circular(2),
                ),
              ),
            ],
          ),
        ],
      ),
      actions: [
        DawContextAction(
          title: "Rename",
          icon: Icons.edit,
          onTap: () {
            final textController = TextEditingController(text: track.name);

            showDialog<String>(
              context: context,
              builder: (ctx) => AlertDialog(
                title: const Text("Rename Track"),
                content: TextField(
                  controller: textController,
                  autofocus: true,
                  decoration: const InputDecoration(
                    labelText: "New track name",
                    border: OutlineInputBorder(),
                  ),
                  onSubmitted: (value) {
                    Navigator.pop(ctx, value);
                  },
                ),
                actions: [
                  TextButton(
                    onPressed: () => Navigator.pop(ctx),
                    child: const Text("Cancel"),
                  ),
                  ElevatedButton(
                    onPressed: () => Navigator.pop(ctx, textController.text),
                    child: const Text("Rename"),
                  ),
                ],
              ),
            ).then((newName) {
              if (newName != null &&
                  newName.trim().isNotEmpty &&
                  newName != track.name) {
                AppLogger.info(
                  "Rename track requested for ID: ${track.id} with name [${newName.trim()}]",
                );
                ref
                    .read(globalStateProvider)
                    .changeTrackName(trackId, newName.trim());
              }
            });
          },
        ),
        DawContextAction(
          title: "Change Color",
          icon: Icons.color_lens,
          onTap: () {
            final currentColor = track.color.toColor();

            _showColorPickerDialog(context, currentColor).then((selectedColor) {
              if (selectedColor != null &&
                  selectedColor.toARGB32() != currentColor.toARGB32()) {
                AppLogger.info(
                  "Change color requested for track ID: ${track.id}",
                );
                ref
                    .read(globalStateProvider)
                    .changeTrackColor(trackId, selectedColor);
              }
            });
          },
        ),
        DawContextAction(
          title: "Move Up",
          icon: Icons.arrow_upward,
          onTap: () async {
            AppLogger.info("Move Up requested for track ID: ${track.id}");
            await handleUpdateTrackOrder(
              ref: ref,
              trackId: trackId,
              newIdx: (track.orderIdx - 1).complyU32(),
            );
          },
        ),
        DawContextAction(
          title: "Move Down",
          icon: Icons.arrow_downward,
          onTap: () async {
            AppLogger.info("Move Down requested for track ID: ${track.id}");
            await handleUpdateTrackOrder(
              ref: ref,
              trackId: trackId,
              newIdx: (track.orderIdx + 1).complyU32(),
            );
          },
        ),
        DawContextAction(
          title: "Delete Track",
          icon: Icons.delete,
          isDestructive: true,
          onTap: () {
            AppLogger.info("Delete track requested for ID: ${track.id}");
            ref.read(globalStateProvider).deleteTrack(trackId);
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
      globalStateProvider.select((s) => s.horizontalZoomLevel),
    );
    final tempo = ref.watch(globalStateProvider.select((s) => s.tempo));
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
    // In Karbeat, 1 beat is always 960 ticks. zoomLevel is in ticks/pixel.
    const double ticksPerBeat = 960.0;
    final double pixelsPerBeat = ticksPerBeat / zoomLevel;

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

class AudioTrackSlot extends ConsumerStatefulWidget {
  final int trackId;
  final double height;
  final ScrollController horizontalScrollController;
  final int sampleRate;
  final ClipDragController clipDragController;

  const AudioTrackSlot({
    super.key,
    required this.trackId,
    required this.height,
    required this.horizontalScrollController,
    required this.sampleRate,
    required this.clipDragController,
  });

  @override
  ConsumerState<AudioTrackSlot> createState() => _AudioTrackSlotState();
}

class _AudioTrackSlotState extends ConsumerState<AudioTrackSlot> {
  void _handleEmptySpaceClick({
    required BuildContext context,
    required double localDx,
    required double zoomLevel,
  }) {
    final state = ref.read(globalStateProvider);
    int startTime = (localDx * zoomLevel).round();

    if (state.snapToGrid) {
      startTime = _snapTick(startTime, state);
    }

    state.createEmptyPatternClip(trackId: widget.trackId, startTime: startTime);
  }

  @override
  Widget build(BuildContext context) {
    // Listen to Zoom Level (Global)
    final zoomLevel = ref.watch(
      globalStateProvider.select((s) => s.horizontalZoomLevel),
    );

    final gridSize = ref.watch(globalStateProvider.select((s) => s.gridSize));
    final tempo = ref.watch(globalStateProvider.select((s) => s.tempo));

    // Listen to Track Data
    final track = ref.watch(
      globalStateProvider.select((s) => s.tracks[widget.trackId]),
    );

    final isSelectedTrack = ref.watch(
      globalStateProvider.select((s) => s.selectedTrackId == widget.trackId),
    );

    final trackSelectedClipIdsStr = ref.watch(
      globalStateProvider.select((s) {
        if (s.selectedTrackId != widget.trackId) return '';
        return s.selectedClipIds.join(',');
      }),
    );

    final safeSampleRate = widget.sampleRate <= 0 ? 48000 : widget.sampleRate;

    final waveformMap = ref.watch(
      trackWaveformProvider((trackId: widget.trackId)),
    );

    final selectedTool = ref.watch(
      globalStateProvider.select((s) => s.selectedTool),
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
                    ref.read(globalStateProvider).deselectAllClips();
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
          ...track.clips.map((clip) {
            final isSelected =
                isSelectedTrack && selectedClipIds.contains(clip.id);

            return _InteractiveClip(
              key: ValueKey(clip.id),
              clip: clip,
              trackId: widget.trackId,
              trackType: track.trackType,
              color: track.color.toColor(),
              zoomLevel: zoomLevel,
              height: widget.height,
              selectedTool: selectedTool,
              isSelected: isSelected,
              selectedClipIds: selectedClipIds,
              clipDragController: widget.clipDragController,
              horizontalScrollController: widget.horizontalScrollController,
              waveformMap: waveformMap,
            );
          }),
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
  final Color color;
  final double zoomLevel;
  final double height;
  final ToolSelection selectedTool;
  final bool isSelected;
  final List<int> selectedClipIds;
  final ClipDragController clipDragController;
  final ScrollController horizontalScrollController;
  final Map<int, WaveformHandle> waveformMap;

  const _InteractiveClip({
    super.key,
    required this.clip,
    required this.trackId,
    required this.trackType,
    required this.zoomLevel,
    required this.height,
    required this.selectedTool,
    required this.color,
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
  double _accumulatedDeltaTicks = 0.0;
  double _verticalDragDy = 0.0;
  int _previousSnappedDelta = 0;
  int _leaderBaseStartTime = 0;
  int _leaderBaseLoopLength = 0;

  // Track vertical drag to determine target track

  /// Track dynamic cursor override
  MouseCursor? _cursorOverride;

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
    final state = ref.read(globalStateProvider);
    final bpm = state.tempo;
    final sr = state.hardwareConfig.sampleRate;
    // Convert to tick-equivalent for rendering on the tick-based timeline
    _visualStartTime = widget.clip.startTimeInTicks(bpm, sr);
    _visualLoopLength = widget.clip.loopLengthInTicks(bpm, sr);
    _visualOffset = widget.clip.offsetStartInTicks(bpm, sr);
  }

  @override
  Widget build(BuildContext context) {
    // Coordinate Mapping (Pixels)
    final double left = _visualStartTime / widget.zoomLevel;
    final double width = _visualLoopLength / widget.zoomLevel;
    final double safeWidth = width < 1 ? 1 : width;
    const resizeEdgeSize = 20.0;

    // final isMoving = _currentAction == _DragAction.move;

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
                  final state = ref.read(globalStateProvider);
                  // If this clip is selected and there are multiple selections, batch delete
                  if (widget.isSelected && widget.selectedClipIds.length > 1) {
                    state.deleteSelectedClips();
                  } else {
                    state.deleteClip(widget.trackId, widget.clip.id);
                  }
                } else if (widget.selectedTool == ToolSelection.select) {
                  final state = ref.read(globalStateProvider);
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
                      .read(globalStateProvider)
                      .selectClip(
                        trackId: widget.trackId,
                        clipId: widget.clip.id,
                      );
                } else if (widget.selectedTool == ToolSelection.slice) {
                  // Calculate absolute position on the timeline (in native clip units)
                  final state = ref.read(globalStateProvider);
                  int cutPoint;
                  if (widget.clip.isSampleBased) {
                    // For audio clips: convert pixel→tick→sample
                    final bpm = state.tempo;
                    final rawSr = state.hardwareConfig.sampleRate;
                    final sr = rawSr > 0 ? rawSr : 48000;
                    int cutTick =
                        widget.clip.startTimeInTicks(bpm, sr) +
                        (details.localPosition.dx * widget.zoomLevel).round();
                    cutTick = _snapClipShiftTick(cutTick, state);
                    // Convert back to samples for the backend
                    cutPoint = ticksToSamples(cutTick, bpm, sr);
                  } else {
                    int cutTick =
                        widget.clip.startTime +
                        (details.localPosition.dx * widget.zoomLevel).round();
                    cutTick = _snapClipShiftTick(cutTick, state);
                    cutPoint = cutTick;
                  }

                  final result = await state.sliceClip(
                    widget.trackId,
                    widget.clip.id,
                    cutPoint,
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

                _accumulatedDeltaTicks = 0.0;
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
                final state = ref.read(globalStateProvider);
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
                      .reduce(
                        (a, b) =>
                            a.startTimeInTicks(
                                  state.tempo,
                                  state.hardwareConfig.sampleRate,
                                ) <
                                b.startTimeInTicks(
                                  state.tempo,
                                  state.hardwareConfig.sampleRate,
                                )
                            ? a
                            : b,
                      );
                  _leaderBaseStartTime = leaderClip.startTimeInTicks(
                    state.tempo,
                    state.hardwareConfig.sampleRate,
                  );
                  _leaderBaseLoopLength = leaderClip.loopLengthInTicks(
                    state.tempo,
                    state.hardwareConfig.sampleRate,
                  );
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

                final state = ref.read(globalStateProvider);
                final currentSelectedIds = state.selectedClipIds;
                final track = state.tracks[widget.trackId];
                if (track == null) return;

                _accumulatedDeltaTicks += details.delta.dx * widget.zoomLevel;
                _verticalDragDy += details.delta.dy;

                int rawTotalDelta = _accumulatedDeltaTicks.round();

                // Safety Clamp: Prevent shrinking past 10 ticks
                final minTicks = 10;
                final shortestClip = track.clips
                    .where((c) => currentSelectedIds.contains(c.id))
                    .reduce((a, b) => a.loopLength < b.loopLength ? a : b);

                if (_currentAction == _DragAction.resizeRight) {
                  final maxShrink = -(shortestClip.loopLength - minTicks);
                  if (rawTotalDelta < maxShrink) {
                    rawTotalDelta = maxShrink.toInt();
                  }
                } else if (_currentAction == _DragAction.resizeLeft) {
                  final maxShrink = shortestClip.loopLength - minTicks;
                  if (rawTotalDelta > maxShrink) {
                    rawTotalDelta = maxShrink.toInt();
                  }
                }

                int snappedTotalDelta = rawTotalDelta;

                // ======================================================
                // Snapping Logic
                // - snapToGrid ON  → clamp to absolute grid border (_snapTick)
                // - snapToGrid OFF → jump in step increments from the clip's
                //   initial position (_snapDeltaToStep), preserving the
                //   initial offset from the nearest step boundary.
                // ======================================================
                if (_currentAction == _DragAction.move ||
                    _currentAction == _DragAction.resizeLeft) {
                  if (state.snapToGrid) {
                    final int rawStart = _leaderBaseStartTime + rawTotalDelta;
                    snappedTotalDelta =
                        _snapTick(rawStart, state) - _leaderBaseStartTime;
                  } else {
                    snappedTotalDelta = _snapDeltaToStep(rawTotalDelta, state);
                  }
                } else if (_currentAction == _DragAction.resizeRight) {
                  if (state.snapToGrid) {
                    final int rawEnd =
                        _leaderBaseStartTime +
                        _leaderBaseLoopLength +
                        rawTotalDelta;
                    snappedTotalDelta =
                        _snapTick(rawEnd, state) -
                        (_leaderBaseStartTime + _leaderBaseLoopLength);
                  } else {
                    snappedTotalDelta = _snapDeltaToStep(rawTotalDelta, state);
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

                final state = ref.read(globalStateProvider);
                final currentSelectedIds = state.selectedClipIds;

                int? newTrackId;
                final rowOffset = (_verticalDragDy / widget.height).round();
                if (rowOffset != 0) {
                  // Sort by visual orderIdx so vertical drag perfectly matches the UI layout
                  final sortedTracksList = state.tracks.values.toList()
                    ..sort((a, b) => a.orderIdx.compareTo(b.orderIdx));
                  final sortedTracks = sortedTracksList
                      .map((t) => t.id)
                      .toList();

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

                final isSampleBased = widget.clip.isSampleBased;
                final hardwareSampleRate = state.hardwareConfig.sampleRate;
                final delta = isSampleBased
                    ? ticksToSamples(
                        _previousSnappedDelta,
                        state.tempo,
                        hardwareSampleRate > 0 ? hardwareSampleRate : 48000,
                      )
                    : _previousSnappedDelta;

                // Send command to Rust backend
                if (_currentAction == _DragAction.move) {
                  state.moveClipBatch(
                    widget.trackId,
                    currentSelectedIds,
                    delta,
                    newTrackId: newTrackId,
                  );
                } else if (_currentAction == _DragAction.resizeRight) {
                  state.resizeClipBatch(
                    widget.trackId,
                    currentSelectedIds,
                    UiResizeEdge.right,
                    delta,
                  );
                } else if (_currentAction == _DragAction.resizeLeft) {
                  state.resizeClipBatch(
                    widget.trackId,
                    currentSelectedIds,
                    UiResizeEdge.left,
                    delta,
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
                color: widget.color,
                zoomLevel: widget.zoomLevel,
                projectSampleRate: ref
                    .read(globalStateProvider)
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
  final Map<int, WaveformHandle> waveformMap;

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
        color: color.withAlpha(100),
        borderRadius: BorderRadius.circular(4),
        border: isSelected
            ? Border.all(color: Colors.white, width: 2)
            : Border.all(color: color.withAlpha(150), width: 1),
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
    final state = ref.watch(globalStateProvider);

    switch (clip.source) {
      case UiClipSource_Audio(:final sourceId):
        final handle = waveformMap[sourceId];
        if (handle == null) {
          return const Center(
            child: Text("Loading...", style: TextStyle(fontSize: 8)),
          );
        }

        final double effectiveOffsetTicks =
            overrideOffset ??
            clip.offsetStartInTicks(state.tempo, projectSampleRate).toDouble();

        // getSampleRate() is a sync opaque call — zero FFI overhead
        final samplesPerTick =
            (60.0 / state.tempo) * (handle.getSampleRate() / 960.0);

        final waveformColor = color.computeLuminance() > 0.5
            ? Colors.black.withAlpha(180) // Dark waveform for light tracks
            : Colors.white.withAlpha(200);

        return RepaintBoundary(
          child: CustomPaint(
            size: Size.infinite,
            painter: StereoWaveformClipPainter(
              // Zero-copy: Float32List view directly into Rust-owned Mmap memory
              samples: createZeroCopyWaveformView(handle),
              color: waveformColor,
              zoomLevel: zoomLevel,
              offsetTicks: effectiveOffsetTicks,
              strokeWidth: 1.0,
              samplesPerTick: samplesPerTick,
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
  final GridSize gridSize;
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
    if (tempo <= 0 ||
        sampleRate <= 0 ||
        zoomLevel <= 0 ||
        gridSize.value <= 0) {
      return;
    }

    // Calculate Grid Dimensions
    final double ticksPerGridLine = (960.0 * 4.0) / gridSize.value;
    double pixelsPerGridLine = ticksPerGridLine / zoomLevel;

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
      // Use the local `position` variable — scrollController.position throws
      // when multiple scroll views share the same controller.
      if (position.hasViewportDimension) {
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

      final int actualGridLines = gridIndex * skipFactor;
      // Is this a bar line?
      // A bar = 4 beats × 960 ticks = 3840 ticks. Use integer tick math to
      // avoid a division-by-zero from the old reciprocal `(1/gridSize.value)`.
      final int ticksAtLine = (actualGridLines * ticksPerGridLine).round();
      final bool isBar = (ticksAtLine % 3840 == 0);

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
    Provider.family<Map<int, WaveformHandle>, ({int trackId})>((ref, arg) {
      // Re-evaluate whenever the track changes (e.g. clips added/removed)
      ref.watch(globalStateProvider.select((s) => s.tracks[arg.trackId]));

      // Sync call — no copy, no await; returns Arc handles into Rust memory
      return getWaveformHandlesForTrack(trackId: arg.trackId);
    });

int computeTargetBin(double zoomLevel) {
  if (zoomLevel <= 1) return 1;

  const levels = [1, 4, 16, 64, 256, 1024];

  for (final l in levels) {
    if (l >= zoomLevel) return l;
  }

  return levels.last; // fallback (max zoomed out)
}

/// Snaps a tick value to the nearest grid line based on the global state
int _snapTick(int ticks, GlobalAppState state) {
  if (!state.snapToGrid) return ticks;

  final gridSize = state.gridSize;

  if (gridSize.value <= 0) return ticks;

  // Calculate the exact tick width of one grid line (4 * 960 = whole note)
  final double ticksPerGridLine = (960.0 * 4.0) / gridSize.value;

  if (ticksPerGridLine <= 0) return ticks;

  // Round to the nearest grid interval
  return ((ticks / ticksPerGridLine).round() * ticksPerGridLine).toInt();
}

/// Snaps an absolute tick value to the nearest global step boundary.
/// Used for the cut tool, where the cut point should land on a step grid line.
int _snapClipShiftTick(int ticks, GlobalAppState state) {
  final step = state.horizontalClipShiftSizeDenom;
  if (step == MusicalBeatSize.none) return ticks;

  final double ticksPerStep = step.value * 960.0;
  if (ticksPerStep <= 0) return ticks;

  return ((ticks / ticksPerStep).round() * ticksPerStep).toInt();
}

/// Snaps a movement **delta** to the nearest multiple of the move-step size.
/// Unlike [_snapClipShiftTick], this does NOT clamp to global grid boundaries.
/// The clip jumps in step-size increments from its initial starting position:
///   new_position = initial_start + round(delta / step) * step
int _snapDeltaToStep(int deltaInTicks, GlobalAppState state) {
  final step = state.horizontalClipShiftSizeDenom;
  if (step == MusicalBeatSize.none) return deltaInTicks;

  final double ticksPerStep = step.value * 960.0;
  if (ticksPerStep <= 0) return deltaInTicks;

  return ((deltaInTicks / ticksPerStep).round() * ticksPerStep).toInt();
}

class _GroupedBatchOverlay extends ConsumerWidget {
  final List<int> trackIds;
  final double itemHeight;
  final ScrollController horizontalScrollController;
  final ScrollController timelineController;
  final ClipDragController clipDragController;

  const _GroupedBatchOverlay({
    required this.trackIds,
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

        final state = ref.read(globalStateProvider);
        final selectedClipIds = state.selectedClipIds;
        final selectedTrackId = state.selectedTrackId;

        if (selectedTrackId == null || selectedClipIds.isEmpty) {
          return const SizedBox();
        }

        final track = state.tracks[selectedTrackId];
        if (track == null) return const SizedBox();

        final zoomLevel = state.horizontalZoomLevel;
        double scrollX = horizontalScrollController.hasClients
            ? horizontalScrollController.offset
            : 0;
        double scrollY = timelineController.hasClients
            ? timelineController.offset
            : 0;

        final trackIndex = trackIds.indexWhere((t) => t == selectedTrackId);
        if (trackIndex < 0) return const SizedBox();

        // Pull the live math directly from the controller
        final snappedDelta = clipDragController.snappedDeltaSamples;
        final verticalDy = clipDragController.verticalDragDy;

        Widget groupedClips = Stack(
          clipBehavior: Clip.none,
          children: selectedClipIds.map((clipId) {
            final clip = track.clips.where((c) => c.id == clipId).firstOrNull;
            if (clip == null) return const SizedBox();

            final bpm = state.tempo;
            final sr = state.hardwareConfig.sampleRate;
            final screenLeft =
                (clip.startTimeInTicks(bpm, sr) / zoomLevel) - scrollX;
            final screenTop = (trackIndex * itemHeight) - scrollY + 30 + 2;
            final clipWidth = clip.loopLengthInTicks(bpm, sr) / zoomLevel;

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

        return Positioned.fill(child: IgnorePointer(child: groupedClips));
      },
    );
  }
}

// ‒‒‒‒‒‒‒‒‒‒‒‒‒‒‒‒‒‒‒‒‒‒‒‒‒‒‒‒‒‒‒‒
// Function to call API from provider
// ‒‒‒‒‒‒‒‒‒‒‒‒‒‒‒‒‒‒‒‒‒‒‒‒‒‒‒‒‒‒‒‒

Future<Result<void>> handleUpdateTrackOrder({
  required WidgetRef ref,
  required int trackId,
  required int newIdx,
}) async {
  return await attemptAsync(() async {
    return await updateTrackOrder(trackId: trackId, newIdx: newIdx);
  }).andThenAsync((_) async {
    await ref.read(globalStateProvider).syncTracksState();
    return Result.ok(null);
  });
}
