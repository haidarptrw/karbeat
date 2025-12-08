import 'package:flutter/material.dart';
import 'package:karbeat/features/components/waveform_painter.dart';
import 'package:karbeat/src/rust/api/project.dart';
import 'package:karbeat/src/rust/core/project.dart';
import 'package:karbeat/state/app_state.dart';
import 'package:provider/provider.dart';

class KarbeatTrackSlot extends StatelessWidget {
  final int trackId;
  final double height;
  final ScrollController horizontalScrollController;

  const KarbeatTrackSlot({
    super.key,
    required this.trackId,
    required this.height,
    required this.horizontalScrollController
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

    final sampleRate = context.select<KarbeatState, int>((state) => state.hardwareConfig.sampleRate);

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
                  sampleRate: sampleRate,
                  scrollController: horizontalScrollController,
                ),
              ),
            ),
          ),
          ...track.clips.map((clip) {
            return _buildClipWidget(context, clip, track.trackType, zoomLevel);
          }),
        ],
      ),
    );
  }

  Widget _buildClipWidget(
    BuildContext context,
    UiClip clip,
    TrackType type,
    double samplesPerPixel,
  ) {
    // === COORDINATE MAPPING ===
    // Start (Pixels) = Start (Samples) / Zoom (Samples per Pixel)
    final double left = clip.startTime / samplesPerPixel;
    final double width = clip.loopLength / samplesPerPixel;

    return Positioned(
      left: left,
      top: 2, // Padding top
      height: height - 4, // Padding bottom
      width: width < 1 ? 1 : width,
      child: _ClipRenderer(
        clip: clip,
        trackType: type,
        // Optional: Pass color based on track or clip settings
        color: Colors.cyanAccent.withAlpha(47),
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

  const _ClipRenderer({
    required this.clip,
    required this.trackType,
    required this.color,
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
        // TODO In future: Use CustomPainter to draw the waveform summary here
        // return const Center(
        //   child: Icon(Icons.graphic_eq, size: 16, color: Colors.white54),
        // );
        return CustomPaint(
          size: Size.infinite, // Fill the clip container
          painter: MonoWaveformPainter(
            samples: field0.previewBuffer, // <--- PASS DATA HERE
            color: Colors.white.withAlpha(200), // High contrast waveform
            strokeWidth: 1.0,
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
  }) : super(repaint: scrollController); // Repaint when scroll changes

  @override
  void paint(Canvas canvas, Size size) {
// 0. Safety Checks
    if (tempo <= 0 || sampleRate <= 0 || zoomLevel <= 0 || gridSize <= 0) return;

    // 1. Calculate Base Interval
    final double samplesPerBeat = (60.0 / tempo) * sampleRate;
    final double samplesPerGridLine = samplesPerBeat * (4.0 / gridSize);
    double pixelsPerGridLine = samplesPerGridLine / zoomLevel;

    // Fix: If grid is effectively 0 (infinite loop risk), stop.
    if (pixelsPerGridLine < 0.0001) return;

    // 2. Adaptive Density (Prevent too many lines)
    int skipFactor = 1;
    // Keep doubling the interval until lines are at least 15px apart
    while (pixelsPerGridLine * skipFactor < 15.0) {
      skipFactor *= 2;
      if (skipFactor > 1000000) break; // Safety break
    }
    
    final double visualInterval = pixelsPerGridLine * skipFactor;

    // 3. Visibility Calculation (Optimization)
    double startX = 0.0;
    double endX = size.width;

    // If attached, only draw what is on screen (+ buffer)
    if (scrollController.hasClients) {
      final position = scrollController.positions.first;
      final double offset = position.pixels;
      // Use fallback width if viewport not ready
      double viewportWidth = size.width;
      if (scrollController.position.hasViewportDimension) {
        viewportWidth = position.viewportDimension;
      }
      
      // Buffer allows scrolling without seeing lines pop in
      const double buffer = 200.0; 
      startX = (offset - buffer).clamp(0.0, double.infinity);
      endX = offset + viewportWidth + buffer;
    }

    // 4. Drawing Loop
    final paint = Paint()
      ..color = Colors.white.withAlpha((0.08*255).round()) // Increased visibility (~8%)
      ..strokeWidth = 1.0;

    final barPaint = Paint()
      ..color = Colors.white.withAlpha((0.25*255).round()) // Increased visibility (~25%)
      ..strokeWidth = 1.0;

    // Calculate first line index
    int gridIndex = (startX / visualInterval).floor();
    double currentX = gridIndex * visualInterval;

    // Iterate until we pass the visible area
    while (currentX < endX) {
      // Don't draw past the container width
      if (currentX > size.width) break;

      // Draw logic
      int actualGridLines = gridIndex * skipFactor;
      bool isBar = (actualGridLines % gridSize == 0); // Logic depends on gridSize definition
      
      // Only draw lines >= 0
      if (currentX >= 0) {
        canvas.drawLine(
          Offset(currentX, 0),
          Offset(currentX, size.height),
          isBar ? barPaint : paint,
        );
      }

      currentX += visualInterval;
      gridIndex++;
    }
  }

  @override
  bool shouldRepaint(covariant _GridPainter oldDelegate) {
    return oldDelegate.zoomLevel != zoomLevel ||
           oldDelegate.gridSize != gridSize ||
           oldDelegate.tempo != tempo ||
           oldDelegate.scrollController != scrollController;
  }
}
