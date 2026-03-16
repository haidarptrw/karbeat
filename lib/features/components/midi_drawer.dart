import 'package:flutter/material.dart';
import 'package:karbeat/src/rust/api/pattern.dart';

class MidiClipPainter extends CustomPainter {
  final UiPattern pattern;
  final Color color;
  final double zoomLevel; // Samples per pixel (Timeline zoom)
  final int sampleRate;
  final double bpm;
  final ScrollController scrollController;
  final double clipLeftOffset;

  MidiClipPainter({
    required this.pattern,
    required this.color,
    required this.zoomLevel,
    required this.sampleRate,
    required this.bpm,
    required this.scrollController,
    required this.clipLeftOffset,
  }) : super(repaint: scrollController);

  @override
  void paint(Canvas canvas, Size size) {
    if (pattern.notes.isEmpty) return;

    final paint = Paint()
      ..color = color.withAlpha((255).round())
      ..style = PaintingStyle.fill;

    double scrollOffset = 0;
    double viewportWidth = 2000;
    if (scrollController.hasClients) {
      scrollOffset = scrollController.offset;
      if (scrollController.position.hasViewportDimension) {
        viewportWidth = scrollController.position.viewportDimension;
      }
    }
    const double pad = 50;
    final double vLeft = (scrollOffset - clipLeftOffset - pad).clamp(
      0,
      size.width,
    );
    final double vRight = (scrollOffset - clipLeftOffset + viewportWidth + pad)
        .clamp(0, size.width);

    const ticksPerBeat = 960.0;

    // Calculate pixels per tick based on global zoom
    final samplesPerTick = (sampleRate * 60.0 / bpm) / ticksPerBeat;
    final pixelsPerTick = samplesPerTick / zoomLevel;

    // Pitch mapping
    int minKey = 127;
    int maxKey = 0;

    for (final note in pattern.notes) {
      if (note.key < minKey) minKey = note.key;
      if (note.key > maxKey) maxKey = note.key;
    }

    // Add padding
    const padding = 2;
    minKey = (minKey - padding).clamp(0, 127);
    maxKey = (maxKey + padding).clamp(0, 127);
    final keyRange = maxKey - minKey;
    final noteHeight = size.height / (keyRange > 0 ? keyRange : 12);

    for (final note in pattern.notes) {
      // X Pos
      final left = note.startTick * pixelsPerTick;
      final width = note.duration * pixelsPerTick;

      // Y Pos (Higher pitch = Upper Y)
      // Invert Y because Canvas 0 is top
      final relativeKey = note.key - minKey;
      final top = size.height - ((relativeKey + 1) * noteHeight);

      // Don't draw if out of bounds (using viewport bounds)
      if (left > vRight) continue;
      if (left + width < vLeft) continue;

      final rect = Rect.fromLTWH(
        left,
        top,
        width < 2 ? 2 : width, // Ensure at least 1px width
        noteHeight - 1, // -1 for spacing
      );

      canvas.drawRect(rect, paint);
    }
  }

  @override
  bool shouldRepaint(covariant MidiClipPainter oldDelegate) {
    return oldDelegate.pattern != pattern ||
        oldDelegate.zoomLevel != zoomLevel ||
        oldDelegate.scrollController != scrollController ||
        oldDelegate.clipLeftOffset != clipLeftOffset ||
        oldDelegate.color != color;
  }
}
