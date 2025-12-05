import 'package:flutter/material.dart';

class StereoWaveformPainter extends CustomPainter {
  final List<double> samples;
  final Color color;

  const StereoWaveformPainter({
    required this.samples,
    this.color = Colors.blueAccent,
  });

  @override
  void paint(Canvas canvas, Size size) {
    if (samples.isEmpty) return;

    final paint = Paint()
      ..color = color
      ..strokeWidth = 1.0
      ..style = PaintingStyle.stroke
      ..strokeCap = StrokeCap.round;

    final centerDividerPaint = Paint()
      ..color = Colors.white.withAlpha(25)
      ..strokeWidth = 1.0;

    // Draw center line
    canvas.drawLine(
      Offset(0, size.height / 2),
      Offset(size.width, size.height / 2),
      centerDividerPaint,
    );

    final path = Path();

    final int totalDataBins = samples.length ~/ 4;

    final double pixels = size.width;
    if (pixels <= 0) return;

    // Data Layout: 4 floats per visual bin [L_min, L_max, R_min, R_max]
    // Total Bins = samples.length / 4
    final int drawSteps = pixels.ceil();
    final double binsPerPixel = totalDataBins / pixels;

    // Height calculations
    final channelHeight = size.height / 2;
    final halfChannelHeight = channelHeight / 2;

    // Y-Centers
    final leftCenterY = halfChannelHeight;
    final rightCenterY = channelHeight + halfChannelHeight;

    for (int i = 0; i < drawSteps; i++) {
      // Determine which data bins correspond to this specific pixel column
      final int startBin = (i * binsPerPixel).floor();
      final int endBin = ((i + 1) * binsPerPixel).ceil();

      // Clamp to data bounds
      final int actualStart = startBin.clamp(0, totalDataBins);
      final int actualEnd = endBin.clamp(0, totalDataBins);

      if (actualStart >= actualEnd) continue;

      // Find the absolute Min/Max in this range (Aggregation)
      double lMin = 1.0;
      double lMax = -1.0;
      double rMin = 1.0;
      double rMax = -1.0;

      for (int i = actualStart; i < actualEnd; i++) {
        final int sampleIdx = i * 4;
        if (sampleIdx + 3 >= samples.length) break;

        final v0 = samples[sampleIdx];
        final v1 = samples[sampleIdx + 1];
        final v2 = samples[sampleIdx + 2];
        final v3 = samples[sampleIdx + 3];

        if (v0 < lMin) lMin = v0;
        if (v1 > lMax) lMax = v1;
        if (v2 < rMin) rMin = v2;
        if (v3 > rMax) rMax = v3;
      }

      // If loop didn't update values (e.g. range mismatch), skip
      if (lMax < lMin) continue;

      final double drawX = i.toDouble();

      // Draw Left Channel (Top) - Invert Y because screen Y goes down
      path.moveTo(drawX, leftCenterY - (lMax * halfChannelHeight));
      path.lineTo(drawX, leftCenterY - (lMin * halfChannelHeight));

      // Draw Right Channel (Bottom)
      path.moveTo(drawX, rightCenterY - (rMax * halfChannelHeight));
      path.lineTo(drawX, rightCenterY - (rMin * halfChannelHeight));
    }

    canvas.drawPath(path, paint);
  }

  @override
  bool shouldRepaint(covariant StereoWaveformPainter oldDelegate) {
    return oldDelegate.samples != samples;
  }
}
