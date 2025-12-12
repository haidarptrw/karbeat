import 'package:flutter/material.dart';
import 'package:karbeat/src/rust/audio/event.dart';
import 'package:karbeat/state/app_state.dart';
import 'package:provider/provider.dart';

class TimelinePlayheadSeeker extends StatefulWidget {
  final double headerWidth;
  final ScrollController scrollController;
  final Function(int samples) onSeek;

  const TimelinePlayheadSeeker({
    super.key,
    required this.headerWidth,
    required this.scrollController,
    required this.onSeek,
  });

  @override
  State<TimelinePlayheadSeeker> createState() => _TimelinePlayheadSeekerState();
}

class _TimelinePlayheadSeekerState extends State<TimelinePlayheadSeeker> {
  late Stream<PlaybackPosition> _positionStream;

  @override
  void initState() {
    super.initState();
    _positionStream = context.read<KarbeatState>().positionStream;
  }

  @override
  void dispose() {
    print("[Flutter] TimelinePlayheadSeeker Disposed");
    // If you are manually listening to a stream (StreamSubscription), you MUST cancel it here.
    // Since you are using StreamBuilder, it handles unsubscription,
    // causing the Rust 'sink.add' to fail, which is exactly what you are seeing.
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final zoomLevel = context.select<KarbeatState, double>(
      (s) => s.horizontalZoomLevel,
    );

    return LayoutBuilder(
      builder: (context, constraints) {
        final viewportWidth = constraints.maxWidth;
        final viewportHeight = constraints.maxHeight;

        // 1. Create a Stack here to serve as the anchor for Positioned
        return Stack(
          clipBehavior: Clip.none,
          children: [
            StreamBuilder<PlaybackPosition>(
              stream: _positionStream,
              builder: (context, snapshot) {
                final currentSamples = snapshot.data?.samples ?? 0;

                double playheadAbsoluteX = 0;
                if (zoomLevel > 0) {
                  playheadAbsoluteX = currentSamples / zoomLevel;
                }

                return AnimatedBuilder(
                  animation: widget.scrollController,
                  builder: (context, child) {
                    double scrollOffset = 0;
                    if (widget.scrollController.hasClients) {
                      scrollOffset = widget.scrollController.offset;
                    }

                    // 2. Calculate Screen X
                    // Header Width + (SongPos - ScrollPos)
                    final double left =
                        widget.headerWidth + playheadAbsoluteX - scrollOffset;

                    // 3. Optimization: Don't render if completely off-screen
                    if (left > viewportWidth + 50) return const SizedBox();

                    // Hide if it goes behind the header (scrolled too far left)
                    if (left < widget.headerWidth) return const SizedBox();

                    // 4. Return the Positioned widget here.
                    // It is now valid because it is inside the Stack at the top of this build method.
                    return Positioned(
                      left: left - 10, // Center the 20px wide handle
                      top: 0,
                      bottom: 0,
                      width: 20, // Hitbox
                      child: GestureDetector(
                        behavior: HitTestBehavior.translucent,
                        onHorizontalDragUpdate: (details) {
                          final deltaPixels = details.delta.dx;
                          final deltaSamples = deltaPixels * zoomLevel;
                          final newSamples = currentSamples + deltaSamples;
                          widget.onSeek(newSamples.toInt());
                        },
                        child: Column(
                          children: [
                            // Triangle
                            SizedBox(
                              height: 20,
                              width: 20,
                              child: CustomPaint(
                                painter: _PlayheadHandlePainter(),
                              ),
                            ),
                            // Line
                            Expanded(
                              child: Container(
                                width: 1.5,
                                color: Colors.yellowAccent.withAlpha(
                                  (0.8 * 255).round(),
                                ),
                              ),
                            ),
                          ],
                        ),
                      ),
                    );
                  },
                );
              },
            ),
          ],
        );
      },
    );
  }
}

class _PlayheadHandlePainter extends CustomPainter {
  @override
  void paint(Canvas canvas, Size size) {
    final paint = Paint()
      ..color = Colors.yellowAccent
      ..style = PaintingStyle.fill;

    final path = Path();
    path.moveTo(0, 0); // Top Left
    path.lineTo(size.width, 0); // Top Right
    path.lineTo(size.width / 2, size.height); // Bottom Center
    path.close();

    canvas.drawPath(path, paint);
    canvas.drawShadow(path, Colors.black, 2.0, false);
  }

  @override
  bool shouldRepaint(covariant CustomPainter oldDelegate) => false;
}
