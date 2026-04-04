import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:karbeat/src/rust/api/audio.dart';
import 'package:karbeat/state/app_state.dart';

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

class PlayheadOverlay extends ConsumerStatefulWidget {
  /// Amount of pixels to offset the draw start (e.g. for Headers)
  final double offsetAdjustment;
  final ScrollController scrollController;
  final Function(int samples) onSeek;

  /// The current zoom level (pixels per sample)
  final double zoomLevel;

  /// Logic to determine which sample count to display (Song vs Pattern)
  final int Function(UiTransportFeedback) sampleSelector;

  const PlayheadOverlay({
    super.key,
    required this.offsetAdjustment,
    required this.scrollController,
    required this.onSeek,
    required this.zoomLevel,
    required this.sampleSelector,
  });

  @override
  ConsumerState<PlayheadOverlay> createState() => _PlayheadOverlayState();
}

class _PlayheadOverlayState extends ConsumerState<PlayheadOverlay> {
  late Stream<UiTransportFeedback> _positionStream;

  bool _isDragging = false;
  int _dragSamples = 0;

  @override
  void initState() {
    super.initState();
    _positionStream = ref.read(karbeatStateProvider).positionStream;
  }

  @override
  Widget build(BuildContext context) {
    return LayoutBuilder(
      builder: (context, constraints) {
        final viewportWidth = constraints.maxWidth;

        return Stack(
          clipBehavior: Clip.none,
          children: [
            StreamBuilder<UiTransportFeedback>(
              stream: _positionStream,
              builder: (context, snapshot) {
                if (!snapshot.hasData) return const SizedBox();

                final streamSamples = widget.sampleSelector(snapshot.data!);

                final currentSamples = _isDragging
                    ? _dragSamples
                    : streamSamples;

                double playheadAbsoluteX = 0;
                if (widget.zoomLevel > 0) {
                  playheadAbsoluteX = currentSamples / widget.zoomLevel;
                }

                return AnimatedBuilder(
                  animation: widget.scrollController,
                  builder: (context, child) {
                    double scrollOffset = 0;
                    if (widget.scrollController.hasClients) {
                      scrollOffset = widget.scrollController.offset;
                    }

                    // Calculate Screen X
                    final double left =
                        widget.offsetAdjustment +
                        playheadAbsoluteX -
                        scrollOffset;

                    // Optimization: Don't render if completely off-screen
                    if (left > viewportWidth + 50) return const SizedBox();

                    // Hide if it goes behind the header/offset (scrolled too far left)
                    if (left < widget.offsetAdjustment) return const SizedBox();

                    return Positioned(
                      left: left - 10, // Center the 20px wide handle
                      top: 0,
                      bottom: 0,
                      width: 20, // Hitbox
                      child: Column(
                        children: [
                          GestureDetector(
                            behavior: HitTestBehavior.opaque,
                            onHorizontalDragStart: (details) {
                              setState(() {
                                _isDragging = true;
                                _dragSamples = streamSamples;
                              });
                            },
                            onHorizontalDragUpdate: (details) {
                              // Update local state instantly for buttery smooth UI
                              setState(() {
                                final deltaSamples = (details.delta.dx * widget.zoomLevel).toInt();
                                _dragSamples += deltaSamples;
                                if (_dragSamples < 0) _dragSamples = 0; // Prevent negative time
                              });
                              
                              widget.onSeek(_dragSamples);
                            },
                            onHorizontalDragEnd: (details) {
                              setState(() {
                                _isDragging = false;
                              });
                            },
                            onHorizontalDragCancel: () {
                              setState(() {
                                _isDragging = false;
                              });
                            },
                            child: SizedBox(
                              height: 20,
                              width: 20,
                              child: CustomPaint(
                                painter: _PlayheadHandlePainter(),
                              ),
                            ),
                          ),
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
