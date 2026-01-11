import 'package:flutter/material.dart';
import 'package:karbeat/features/components/midi_drawer.dart';
import 'package:karbeat/features/components/waveform_painter.dart';
import 'package:karbeat/features/playlist/clip_drag_controller.dart';
import 'package:karbeat/src/rust/api/project.dart';
import 'package:karbeat/src/rust/api/track.dart';
import 'package:karbeat/src/rust/core/project/track.dart';
import 'package:karbeat/state/app_state.dart';
import 'package:provider/provider.dart';

class KarbeatTrackSlot extends StatefulWidget {
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
  State<KarbeatTrackSlot> createState() => _KarbeatTrackSlotState();
}

class _KarbeatTrackSlotState extends State<KarbeatTrackSlot> {
  void _handleEmptySpaceClick({
    required BuildContext context,
    required double localDx,
    required double zoomLevel,
  }) {
    final int startTime = (localDx * zoomLevel).round();

    context.read<KarbeatState>().createEmptyPatternClip(
      trackId: widget.trackId,
      startTime: startTime,
    );
  }

  @override
  Widget build(BuildContext context) {
    // Listen to Zoom Level (Global)
    final zoomLevel = context.select<KarbeatState, double>(
      (state) => state.horizontalZoomLevel,
    );

    final gridSize = context.select<KarbeatState, int>(
      (state) => state.gridSize,
    );
    final tempo = context.select<KarbeatState, double>((state) => state.tempo);

    // Listen to Track Data
    final track = context.select<KarbeatState, UiTrack?>(
      (state) => state.tracks[widget.trackId],
    );

    final safeSampleRate = widget.sampleRate <= 0 ? 48000 : widget.sampleRate;

    final selectedTool = context.select<KarbeatState, ToolSelection>(
      (s) => s.selectedTool,
    );

    final selectedClipIds = context.select<KarbeatState, List<int>>(
      (state) => state.sessionState?.selectedClipIds ?? [],
    );
    final selectedTrackId = context.select<KarbeatState, int?>(
      (state) => state.sessionState?.selectedTrackId,
    );

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
                    context.read<KarbeatState>().deselectAllClips();
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
                (selectedTrackId != null) &&
                (widget.trackId == selectedTrackId &&
                    selectedClipIds.contains(clip.id));
            return _InteractiveClip(
              key: ValueKey(
                // Important for performance/state retention
                clip.id,
              ),
              clip: clip,
              trackId: widget.trackId,
              trackType: track.trackType,
              zoomLevel: zoomLevel,
              height: widget.height,
              selectedTool: selectedTool,
              isSelected: isSelected,
              selectedClipIds: selectedClipIds,
              clipDragController: widget.clipDragController,
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

class _InteractiveClip extends StatefulWidget {
  final UiClip clip;
  final int trackId;
  final TrackType trackType;
  final double zoomLevel;
  final double height;
  final ToolSelection selectedTool;
  final bool isSelected;
  final List<int> selectedClipIds;
  final ClipDragController clipDragController;

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
  });

  @override
  State<_InteractiveClip> createState() => _InteractiveClipState();
}

enum _DragAction { none, resizeLeft, resizeRight, move }

class _InteractiveClipState extends State<_InteractiveClip> {
  // Local state for smooth UI updates during drag
  late int _visualStartTime;
  late int _visualLoopLength;
  late int _visualOffset;

  _DragAction _currentAction = _DragAction.none;

  // Track vertical drag to determine target track
  double _verticalDragDy = 0.0;

  /// Track dynamic cursor override
  MouseCursor? _cursorOverride;

  // Overlay for global dragging
  OverlayEntry? _overlayEntry;
  final ValueNotifier<Offset> _overlayPosition = ValueNotifier(Offset.zero);

  // Track base values for follower sync
  int _baseStartTime = 0;
  int _baseLoopLength = 0;
  int _baseOffset = 0;

  @override
  void initState() {
    super.initState();
    _syncModel();
    // Listen to batch drag updates for follower visual sync
    widget.clipDragController.addListener(_onBatchDragUpdate);
  }

  @override
  void dispose() {
    widget.clipDragController.removeListener(_onBatchDragUpdate);
    super.dispose();
  }

  @override
  void didUpdateWidget(covariant _InteractiveClip oldWidget) {
    super.didUpdateWidget(oldWidget);
    // Re-attach listener if controller changed
    if (oldWidget.clipDragController != widget.clipDragController) {
      oldWidget.clipDragController.removeListener(_onBatchDragUpdate);
      widget.clipDragController.addListener(_onBatchDragUpdate);
    }
    // Only overwrite local state from backend if we are NOT currently dragging
    // and not in a batch drag as a follower
    if (_currentAction == _DragAction.none && !_isFollower) {
      _syncModel();
    }
  }

  /// Check if this clip is a follower in a batch drag (selected but not leader)
  bool get _isFollower {
    final controller = widget.clipDragController;
    return controller.isActive &&
        widget.isSelected &&
        controller.leaderClipId != widget.clip.id;
  }

  /// Handle batch drag updates for follower clips
  void _onBatchDragUpdate() {
    if (!_isFollower) return;

    final controller = widget.clipDragController;

    setState(() {
      switch (controller.action) {
        case BatchDragAction.move:
          _visualStartTime = (_baseStartTime + controller.deltaSamples).clamp(
            0,
            double.maxFinite.toInt(),
          );
          break;
        case BatchDragAction.resizeRight:
          _visualLoopLength = (_baseLoopLength + controller.deltaSamples).clamp(
            100,
            double.maxFinite.toInt(),
          );
          break;
        case BatchDragAction.resizeLeft:
          final oldEnd = _baseStartTime + _baseLoopLength;
          final newStart = (_baseStartTime + controller.deltaSamples).clamp(
            0,
            oldEnd - 100,
          );
          final moveAmount = newStart - _baseStartTime;
          _visualStartTime = newStart;
          _visualLoopLength = oldEnd - newStart;
          _visualOffset = (_baseOffset + moveAmount).clamp(
            0,
            double.maxFinite.toInt(),
          );
          break;
        case BatchDragAction.none:
          // Reset to base when drag ends
          _syncModel();
          break;
      }
    });
  }

  void _syncModel() {
    _visualStartTime = widget.clip.startTime.toInt();
    _visualLoopLength = widget.clip.loopLength.toInt();
    _visualOffset = widget.clip.offsetStart.toInt();
    _verticalDragDy = 0.0;
    // Store base values for follower calculations
    _baseStartTime = _visualStartTime;
    _baseLoopLength = _visualLoopLength;
    _baseOffset = _visualOffset;
  }

  void _createOverlay(BuildContext context) {
    final renderBox = context.findRenderObject() as RenderBox?;
    if (renderBox == null) return;
    final size = renderBox.size;
    final initialGlobalPos = renderBox.localToGlobal(Offset.zero);
    _overlayPosition.value = initialGlobalPos;

    // Create entry
    _overlayEntry = OverlayEntry(
      builder: (context) {
        return ValueListenableBuilder<Offset>(
          valueListenable: _overlayPosition,
          builder: (context, offset, child) {
            return Positioned(
              left: offset.dx,
              top: offset.dy,
              width: size.width,
              height: size.height,
              child: Material(
                color: Colors.transparent,
                child: _ClipRenderer(
                  clip: widget.clip,
                  trackType: widget.trackType,
                  color: Colors.cyanAccent.withAlpha((0.5 * 255).round()),
                  zoomLevel: widget.zoomLevel,
                  projectSampleRate: context
                      .read<KarbeatState>()
                      .hardwareConfig
                      .sampleRate,
                  overrideOffset: _visualOffset.toDouble(),
                  isSelected: widget.isSelected,
                ),
              ),
            );
          },
        );
      },
    );

    Overlay.of(context).insert(_overlayEntry!);
  }

  void _updateOverlay(Offset delta) {
    _overlayPosition.value += delta;
  }

  void _removeOverlay() {
    _overlayEntry?.remove();
    _overlayEntry = null;
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
    }

    // Apply Override
    if (_cursorOverride != null) {
      cursor = _cursorOverride!;
    }

    // Check if this is a follower in a batch move (should be semi-transparent)
    final isFollowerInBatchMove =
        _isFollower && widget.clipDragController.action == BatchDragAction.move;

    return Positioned(
      left: left,
      top: 2,
      height: widget.height - 4,
      width: safeWidth,
      child: Opacity(
        // Leader becomes invisible (has overlay), followers become semi-transparent
        opacity: isMoving ? 0.0 : (isFollowerInBatchMove ? 0.4 : 1.0),
        child: MouseRegion(
          cursor: cursor,
          // Detect Hover for Resize Cursors (only if not in Delete mode)
          onHover: (event) {
            if (widget.selectedTool == ToolSelection.delete) return;

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

            onTap: () {
              if (widget.selectedTool == ToolSelection.delete) {
                final state = context.read<KarbeatState>();
                // If this clip is selected and there are multiple selections, batch delete
                if (widget.isSelected && widget.selectedClipIds.length > 1) {
                  state.deleteSelectedClips();
                } else {
                  state.deleteClip(widget.trackId, widget.clip.id);
                }
              } else if (widget.selectedTool == ToolSelection.pointer) {
                context.read<KarbeatState>().selectClip(
                  trackId: widget.trackId,
                  clipId: widget.clip.id,
                );
              }
            },

            onPanStart: (details) {
              if (widget.selectedTool == ToolSelection.delete) return;

              final x = details.localPosition.dx;

              if (x < resizeEdgeSize) {
                setState(() => _currentAction = _DragAction.resizeLeft);
              } else if (x > safeWidth - resizeEdgeSize) {
                setState(() => _currentAction = _DragAction.resizeRight);
              } else {
                setState(() => _currentAction = _DragAction.move);
                _createOverlay(context);
              }

              // Start batch drag if this clip is selected and has siblings
              if (widget.isSelected && widget.selectedClipIds.length > 1) {
                final batchAction = _currentAction == _DragAction.move
                    ? BatchDragAction.move
                    : _currentAction == _DragAction.resizeLeft
                    ? BatchDragAction.resizeLeft
                    : _currentAction == _DragAction.resizeRight
                    ? BatchDragAction.resizeRight
                    : BatchDragAction.none;
                widget.clipDragController.startBatchDrag(
                  widget.clip.id,
                  batchAction,
                );
              }
            },

            onPanUpdate: (details) {
              if (_currentAction == _DragAction.none) return;
              final deltaSamples = (details.delta.dx * widget.zoomLevel)
                  .round();

              // Update batch controller delta for followers
              if (widget.isSelected && widget.selectedClipIds.length > 1) {
                widget.clipDragController.updateDelta(
                  deltaSamples,
                  details.delta.dy / widget.height,
                );
              }

              if (_currentAction == _DragAction.move) {
                _updateOverlay(details.delta);

                setState(() {
                  _visualStartTime = (_visualStartTime + deltaSamples)
                      .clamp(0, double.infinity)
                      .toInt();
                  _verticalDragDy += details.delta.dy;
                });
              } else {
                setState(() {
                  if (_currentAction == _DragAction.resizeRight) {
                    _visualLoopLength = (_visualLoopLength + deltaSamples)
                        .clamp(100, double.infinity)
                        .toInt();
                  } else if (_currentAction == _DragAction.resizeLeft) {
                    final oldEnd = _visualStartTime + _visualLoopLength;
                    final newStart = (_visualStartTime + deltaSamples)
                        .clamp(0, oldEnd - 100)
                        .toInt();

                    final moveAmount = newStart - _visualStartTime;
                    _visualStartTime = newStart;
                    _visualLoopLength = oldEnd - newStart;
                    _visualOffset += moveAmount;
                    if (_visualOffset < 0) _visualOffset = 0;
                  }
                });
              }
            },

            onPanEnd: (_) {
              if (_currentAction == _DragAction.none) return;

              final state = context.read<KarbeatState>();
              final isBatchOperation =
                  widget.isSelected && widget.selectedClipIds.length > 1;
              final controller = widget.clipDragController;

              if (_currentAction == _DragAction.move) {
                _removeOverlay();
                int? newTrackId;

                // Estimate row index offset based on drag distance and row height
                final rowOffset = (_verticalDragDy / widget.height).round();

                if (rowOffset != 0) {
                  // Find target track ID from state list
                  final sortedTracks = state.tracks.values.toList()
                    ..sort((a, b) => a.id.compareTo(b.id));

                  final currentIndex = sortedTracks.indexWhere(
                    (t) => t.id == widget.trackId,
                  );
                  if (currentIndex != -1) {
                    final targetIndex = currentIndex + rowOffset;
                    if (targetIndex >= 0 && targetIndex < sortedTracks.length) {
                      newTrackId = sortedTracks[targetIndex].id;
                    }
                  }
                }

                if (isBatchOperation) {
                  // Batch move using delta
                  state.moveClipBatch(
                    widget.trackId,
                    widget.selectedClipIds,
                    controller.deltaSamples,
                    newTrackId: newTrackId,
                  );
                } else {
                  state.moveClip(
                    widget.trackId,
                    widget.clip.id,
                    _visualStartTime,
                    newTrackId: newTrackId,
                  );
                }
              } else if (_currentAction == _DragAction.resizeRight) {
                if (isBatchOperation) {
                  state.resizeClipBatch(
                    widget.trackId,
                    widget.selectedClipIds,
                    ResizeEdge.right,
                    controller.deltaSamples,
                  );
                } else {
                  final newEndTime = _visualStartTime + _visualLoopLength;
                  state.resizeClip(
                    widget.trackId,
                    widget.clip.id,
                    ResizeEdge.right,
                    newEndTime,
                  );
                }
              } else if (_currentAction == _DragAction.resizeLeft) {
                if (isBatchOperation) {
                  state.resizeClipBatch(
                    widget.trackId,
                    widget.selectedClipIds,
                    ResizeEdge.left,
                    controller.deltaSamples,
                  );
                } else {
                  state.resizeClip(
                    widget.trackId,
                    widget.clip.id,
                    ResizeEdge.left,
                    _visualStartTime,
                  );
                }
              }

              // Reset batch controller
              if (isBatchOperation) {
                controller.reset();
              }

              // Reset
              setState(() {
                _currentAction = _DragAction.none;
                _verticalDragDy =
                    0.0; // Snap back visually until state sync updates
              });
            },

            child: _ClipRenderer(
              clip: widget.clip,
              trackType: widget.trackType,
              color: Colors.cyanAccent.withAlpha(47),
              zoomLevel: widget.zoomLevel,
              projectSampleRate: context
                  .read<KarbeatState>()
                  .hardwareConfig
                  .sampleRate,
              overrideOffset: _visualOffset.toDouble(),
              isSelected: widget.isSelected,
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

class _ClipRenderer extends StatelessWidget {
  final UiClip clip;
  final TrackType trackType;
  final Color color;
  final double zoomLevel;
  final int projectSampleRate;
  final double? overrideOffset;
  final bool isSelected;

  const _ClipRenderer({
    required this.clip,
    required this.trackType,
    required this.color,
    required this.zoomLevel,
    required this.projectSampleRate,
    this.overrideOffset,
    required this.isSelected,
  });

  @override
  Widget build(BuildContext context) {
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
            Positioned.fill(child: _buildContent(context)),

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

  Widget _buildContent(BuildContext context) {
    final state = context.read<KarbeatState>();

    switch (clip.source) {
      // In future: Use CustomPainter to draw the waveform summary here

      case UiClipSource_Audio(:final sourceId):
        double ratio = 1.0;
        final audioData = state.audioSources[sourceId];
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

        return CustomPaint(
          size: Size.infinite, // Fill the clip container
          painter: StereoWaveformClipPainter(
            samples: audioData.previewBuffer,
            color: Colors.white.withAlpha(200),
            zoomLevel: zoomLevel,
            offsetSamples: effectiveOffset,
            strokeWidth: 1.0,
            ratio: ratio,
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

        return CustomPaint(
          size: Size.infinite,
          painter: MidiClipPainter(
            pattern: pattern,
            color: color,
            zoomLevel: zoomLevel,
            sampleRate: projectSampleRate,
            bpm: state.transport.bpm,
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
