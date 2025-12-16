import 'package:flutter/material.dart';
import 'package:karbeat/src/rust/api/pattern.dart';

class MidiClipPainter extends CustomPainter {
  final UiPattern pattern;
  final Color color;
  final double zoomLevel; // Samples per pixel (Timeline zoom)
  final int sampleRate;
  final double bpm;

  MidiClipPainter({
    super.repaint,
    required this.pattern,
    required this.color,
    required this.zoomLevel,
    required this.sampleRate,
    required this.bpm,
  });

  @override
  void paint(Canvas canvas, Size size) {
    if (pattern.notes.isEmpty) return;

    final paint = Paint()
      ..color = color.withAlpha((0.9 * 255).round())
      ..style = PaintingStyle.fill;

    const ticksPerBeat = 960.0;

    // Calculate pixels per tick based on global zoom
    final samplesPerTick = (sampleRate * 60.0 / bpm) / ticksPerBeat;
    final pixelsPerTick = samplesPerTick / zoomLevel;

    // Pitch mapping
    int minKey = 0;
    int maxKey = 127;
    for (final note in pattern.notes) {
      if (note.key < minKey) minKey = note.key;
      if (note.key > maxKey) maxKey = note.key;
    }

    // Add padding
    minKey = (minKey - 5).clamp(0, 127);
    maxKey = (maxKey + 5).clamp(0, 127);
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

      // Don't draw if out of bounds
      if (left > size.width) continue;
      if (left + width < 0) continue;

      final rect = Rect.fromLTWH(
        left,
        top,
        width < 1 ? 1 : width, // Ensure at least 1px width
        noteHeight - 1, // -1 for spacing
      );

      canvas.drawRect(rect, paint);
    }
  }

  @override
  bool shouldRepaint(covariant MidiClipPainter oldDelegate) {
    return oldDelegate.pattern != pattern ||
           oldDelegate.zoomLevel != zoomLevel ||
           oldDelegate.color != color;
  }
}
