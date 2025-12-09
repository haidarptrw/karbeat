import 'package:flutter/material.dart';
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
          ...track.clips.map((clip) {
            // return _buildClipWidget(context, clip, track.trackType, zoomLevel);
            return _InteractiveClip(
              key: ValueKey(
                clip.id,
              ), // Important for performance/state retention
              clip: clip,
              trackId: trackId,
              trackType: track.trackType,
              zoomLevel: zoomLevel,
              height: height,
              selectedTool: selectedTool,
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

  const _InteractiveClip({
    super.key,
    required this.clip,
    required this.trackId,
    required this.trackType,
    required this.zoomLevel,
    required this.height,
    required this.selectedTool,
  });

  @override
  State<_InteractiveClip> createState() => _InteractiveClipState();
}

enum _DragAction { none, resizeLeft, resizeRight }

class _InteractiveClipState extends State<_InteractiveClip> {
  _DragAction _currentAction = _DragAction.none;

  // Local state for smooth UI updates during drag
  late int _visualStartTime;
  late int _visualLoopLength;

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
  }

  @override
  Widget build(BuildContext context) {
    // Coordinate Mapping (Pixels)
    final double left = _visualStartTime / widget.zoomLevel;
    final double width = _visualLoopLength / widget.zoomLevel;
    final double safeWidth = width < 1 ? 1 : width;

    // Determine Cursor
    MouseCursor cursor = SystemMouseCursors.basic;
    if (widget.selectedTool == ToolSelection.delete) {
      cursor = SystemMouseCursors.click; // Or use SystemMouseCursors.forbidden
    }
    // For normal pointers, we determine resize cursor in MouseRegion logic below
    // but we can set a default here.

    return Positioned(
      left: left,
      top: 2,
      height: widget.height - 4,
      width: safeWidth,
      child: MouseRegion(
        cursor: cursor,
        // Detect Hover for Resize Cursors (only if not in Delete mode)
        onHover: (event) {
          if (widget.selectedTool == ToolSelection.delete) return;

          final x = event.localPosition.dx;
          const edgeSize = 10.0;
          if (x < edgeSize || x > safeWidth - edgeSize) {
            // TODO: Ideally use a ValueNotifier to switch cursor to resizeLeftRight
            // For now, standard cursor is fine or implementation specific
          }
        },
        child: GestureDetector(
          // Opaque ensures we catch taps even on transparent parts of waveform
          behavior: HitTestBehavior.opaque,

          // --- 1. DELETE ACTION ---
          onTap: () {
            if (widget.selectedTool == ToolSelection.delete) {
              context.read<KarbeatState>().deleteClip(
                widget.trackId,
                widget.clip.id,
              );
            }
          },

          // --- 2. DRAG START (RESIZE) ---
          onHorizontalDragStart: (details) {
            // Disable interactions if Delete tool is active
            if (widget.selectedTool == ToolSelection.delete) return;

            final x = details.localPosition.dx;
            const edgeSize = 15.0; // Hitbox for resizing

            if (x < edgeSize) {
              setState(() => _currentAction = _DragAction.resizeLeft);
            } else if (x > safeWidth - edgeSize) {
              setState(() => _currentAction = _DragAction.resizeRight);
            } else {
              // Middle click - Skip Move as requested
              _currentAction = _DragAction.none;
            }
          },

          // --- 3. DRAG UPDATE (VISUAL) ---
          onHorizontalDragUpdate: (details) {
            if (_currentAction == _DragAction.none) return;

            // Convert pixel delta to sample delta
            final deltaSamples = (details.delta.dx * widget.zoomLevel).round();

            setState(() {
              if (_currentAction == _DragAction.resizeRight) {
                // Changing Length
                _visualLoopLength = (_visualLoopLength + deltaSamples)
                    .clamp(100, double.infinity)
                    .toInt();
              } else if (_currentAction == _DragAction.resizeLeft) {
                // Changing Start + Length (Visual Slip)
                final oldEnd = _visualStartTime + _visualLoopLength;

                // New start cannot exceed old end
                final newStart = (_visualStartTime + deltaSamples)
                    .clamp(0, oldEnd - 100)
                    .toInt();

                _visualStartTime = newStart;
                _visualLoopLength = oldEnd - newStart;
              }
            });
          },

          // --- 4. DRAG END (COMMIT) ---
          onHorizontalDragEnd: (_) {
            if (_currentAction == _DragAction.none) return;

            final state = context.read<KarbeatState>();

            if (_currentAction == _DragAction.resizeRight) {
              // API expects absolute NEW TIME value for edge
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

            setState(() => _currentAction = _DragAction.none);
          },

          // --- 5. VISUAL RENDERER ---
          child: _ClipRenderer(
            clip: widget.clip,
            trackType: widget.trackType,
            color: Colors.cyanAccent.withAlpha(47),
            zoomLevel: widget.zoomLevel,
            projectSampleRate: context.read<KarbeatState>().hardwareConfig.sampleRate,
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

  const _ClipRenderer({
    required this.clip,
    required this.trackType,
    required this.color,
    required this.zoomLevel,
    required this.projectSampleRate,
  });

  @override
  Widget build(BuildContext context) {
    return Container(
      decoration: BoxDecoration(
        color: color,
        borderRadius: BorderRadius.circular(4),
        border: Border.all(color: color.withAlpha(16), width: 1),
      ),
      child: ClipRRect(
        borderRadius: BorderRadius.circular(3),
        child: Stack(
          children: [
            // A. Content (Waveform or MIDI Notes)
            Positioned.fill(child: _buildContent()),

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

  Widget _buildContent() {
    switch (clip.source) {
      // In future: Use CustomPainter to draw the waveform summary here

      // case KarbeatSource_Midi():
      //   return const Center(
      //     child: Icon(Icons.piano, size: 16, color: Colors.white54),
      //   );
      // // In future: Draw mini rectangles for notes

      // case KarbeatSource_Automation():
      //   return const Center(
      //     child: Icon(Icons.show_chart, size: 16, color: Colors.white54),
      //   );
      case UiClipSource_Audio(:final field0):
        double ratio = 1.0;
        if (projectSampleRate > 0 && field0.sampleRate > 0) {
          ratio = field0.sampleRate / projectSampleRate;
        }
        return CustomPaint(
          size: Size.infinite, // Fill the clip container
          painter: StereoWaveformClipPainter(
            samples: field0.previewBuffer,
            color: Colors.white.withAlpha(200),
            zoomLevel: zoomLevel,
            offsetSamples: clip.offsetStart.toDouble(),
            strokeWidth: 1.0,
            ratio: ratio
          ),
        );
      case UiClipSource_None():
        return const Center(
          child: Icon(Icons.show_chart, size: 16, color: Colors.white54),
        );
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
