import 'package:flutter/material.dart';
import 'package:karbeat/features/components/midi_drawer.dart';
import 'package:karbeat/features/components/waveform_painter.dart';
import 'package:karbeat/src/rust/api/project.dart';
import 'package:karbeat/src/rust/api/track.dart';
import 'package:karbeat/src/rust/core/project.dart';
import 'package:karbeat/state/app_state.dart';
import 'package:provider/provider.dart';

class KarbeatTrackSlot extends StatelessWidget {
  final int trackId;
  final double height;
  final ScrollController horizontalScrollController;
  final int sampleRate;

  const KarbeatTrackSlot({
    super.key,
    required this.trackId,
    required this.height,
    required this.horizontalScrollController,
    required this.sampleRate,
  });

  void _handleEmptySpaceClick({
    required BuildContext context,
    required double localDx,
    required double zoomLevel,
  }) {
    final int startTime = (localDx * zoomLevel).round();

    context.read<KarbeatState>().createEmptyPatternClip(
      trackId: trackId,
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
      (state) => state.tracks[trackId],
    );

    final safeSampleRate = sampleRate <= 0 ? 48000 : sampleRate;

    final selectedTool = context.select<KarbeatState, ToolSelection>(
      (s) => s.selectedTool,
    );

    final selectedClipId = context.select<KarbeatState, int?>(
      (state) => state.sessionState?.selectedClipId,
    );
    final selectedTrackId = context.select<KarbeatState, int?>(
      (state) => state.sessionState?.selectedTrackId,
    );

    if (track == null) return const SizedBox();

    return Container(
      height: height,
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
                    context.read<KarbeatState>().deselectClip();
                  }
                },
                child: RepaintBoundary(
                  child: CustomPaint(
                    painter: _GridPainter(
                      zoomLevel: zoomLevel,
                      gridSize: gridSize,
                      tempo: tempo,
                      sampleRate: safeSampleRate,
                      scrollController: horizontalScrollController,
                    ),
                  ),
                ),
              ),
            ),
          ),
          ...track.clips.map((clip) {
            final isSelected =
                (selectedTrackId != null) &&
                (selectedClipId != null) &&
                (trackId == selectedTrackId && clip.id == selectedClipId);
            return _InteractiveClip(
              key: ValueKey(
                // Important for performance/state retention
                clip.id,
              ),
              clip: clip,
              trackId: trackId,
              trackType: track.trackType,
              zoomLevel: zoomLevel,
              height: height,
              selectedTool: selectedTool,
              isSelected: isSelected,
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

  const _InteractiveClip({
    super.key,
    required this.clip,
    required this.trackId,
    required this.trackType,
    required this.zoomLevel,
    required this.height,
    required this.selectedTool,
    required this.isSelected,
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

  // Overlay for global draggin
  OverlayEntry? _overlayEntry;
  final ValueNotifier<Offset> _overlayPosition = ValueNotifier(Offset.zero);

  @override
  void initState() {
    super.initState();
    _syncModel();
  }

  @override
  void didUpdateWidget(covariant _InteractiveClip oldWidget) {
    super.didUpdateWidget(oldWidget);
    // Only overwrite local state from backend if we are NOT currently dragging
    if (_currentAction == _DragAction.none) {
      _syncModel();
    }
  }

  void _syncModel() {
    _visualStartTime = widget.clip.startTime.toInt();
    _visualLoopLength = widget.clip.loopLength.toInt();
    _visualOffset = widget.clip.offsetStart.toInt();
    _verticalDragDy = 0.0;
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
    const resizeEdgeSize = 15.0;

    final isMoving = _currentAction == _DragAction.move;

    final double top = 2 + _verticalDragDy;

    // Determine Cursor
    MouseCursor cursor = SystemMouseCursors.basic;
    if (widget.selectedTool == ToolSelection.delete) {
      cursor = SystemMouseCursors.click;
    } else if (widget.selectedTool == ToolSelection.move) {
      cursor = SystemMouseCursors.move;
    }

    return Positioned(
      left: left,
      top: 2,
      height: widget.height - 4,
      width: safeWidth,
      child: Opacity(
        opacity: isMoving ? 0.0 : 1.0,
        child: MouseRegion(
          cursor: cursor,
          // Detect Hover for Resize Cursors (only if not in Delete mode)
          onHover: (event) {
            if (widget.selectedTool == ToolSelection.delete) return;

            final x = event.localPosition.dx;
            if (x < resizeEdgeSize || x > safeWidth - resizeEdgeSize) {
              // TODO: Ideally use a ValueNotifier to switch cursor to resizeLeftRight
              // For now, standard cursor is fine or implementation specific
            }
          },
          child: GestureDetector(
            // Opaque ensures we catch taps even on transparent parts of waveform
            behavior: HitTestBehavior.opaque,

            onTap: () {
              if (widget.selectedTool == ToolSelection.delete) {
                context.read<KarbeatState>().deleteClip(
                  widget.trackId,
                  widget.clip.id,
                );
              } else if (widget.selectedTool == ToolSelection.pointer) {
                context.read<KarbeatState>().updateSelectedClip(
                  trackId: widget.trackId,
                  clipId: widget.clip.id,
                );
              }
            },

            onPanStart: (details) {
              if (widget.selectedTool == ToolSelection.delete) return;

              final x = details.localPosition.dx;

              if (widget.selectedTool == ToolSelection.move) {
                if (x < resizeEdgeSize) {
                  setState(() => _currentAction = _DragAction.resizeLeft);
                } else if (x > safeWidth - resizeEdgeSize) {
                  setState(() => _currentAction = _DragAction.resizeRight);
                } else {
                  setState(() => _currentAction = _DragAction.move);
                  _createOverlay(context);
                }
              }
            },

            onPanUpdate: (details) {
              if (_currentAction == _DragAction.none) return;
              final deltaSamples = (details.delta.dx * widget.zoomLevel)
                  .round();

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

              if (_currentAction == _DragAction.move) {
                _removeOverlay();
                int? newTrackId;

                // Estimate row index offset based on drag distance and row height
                final rowOffset = (_verticalDragDy / widget.height).round();

                if (rowOffset != 0) {
                  // Find target track ID from state list
                  // We need the ordered list of tracks to know who is above/below
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

                state.moveClip(
                  widget.trackId,
                  widget.clip.id,
                  _visualStartTime,
                  newTrackId:
                      newTrackId, // Pass the new track (or null if same)
                );
              } else if (_currentAction == _DragAction.resizeRight) {
                final newEndTime = _visualStartTime + _visualLoopLength;
                state.resizeClip(
                  widget.trackId,
                  widget.clip.id,
                  ResizeEdge.right,
                  newEndTime,
                );
              } else if (_currentAction == _DragAction.resizeLeft) {
                state.resizeClip(
                  widget.trackId,
                  widget.clip.id,
                  ResizeEdge.left,
                  _visualStartTime,
                );
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
        border: isSelected ? Border.all(color: Colors.white, width:  2) : Border.all(color: color.withAlpha(16), width: 1),
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
