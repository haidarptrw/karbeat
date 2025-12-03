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
      ..strokeWidth = 1.5
      ..style = PaintingStyle.stroke
      ..strokeCap = StrokeCap.round;

    final centerDividerPaint = Paint()
      ..color = Colors.white.withAlpha(25)
      ..strokeWidth = 1.0;

    // Draw center line
    canvas.drawLine(
        Offset(0, size.height / 2), Offset(size.width, size.height / 2), centerDividerPaint);

    final path = Path();

    // Data Layout: 4 floats per visual bin [L_min, L_max, R_min, R_max]
    // Total Bins = samples.length / 4
    final stepX = size.width / (samples.length / 4);

    // Height calculations
    final channelHeight = size.height / 2;
    final halfChannelHeight = channelHeight / 2;
    
    // Y-Centers
    final leftCenterY = halfChannelHeight; 
    final rightCenterY = channelHeight + halfChannelHeight;

    for (int i = 0; i < samples.length; i += 4) {
      if (i + 3 >= samples.length) break;

      // Extract raw values (-1.0 to 1.0)
      final lMin = samples[i];
      final lMax = samples[i + 1];
      final rMin = samples[i + 2];
      final rMax = samples[i + 3];

      final x = (i / 4) * stepX;

      // Draw Left Channel (Top) - Invert Y because screen Y goes down
      path.moveTo(x, leftCenterY - (lMax * halfChannelHeight));
      path.lineTo(x, leftCenterY - (lMin * halfChannelHeight));

      // Draw Right Channel (Bottom)
      path.moveTo(x, rightCenterY - (rMax * halfChannelHeight));
      path.lineTo(x, rightCenterY - (rMin * halfChannelHeight));
    }

    canvas.drawPath(path, paint);
  }

  @override
  bool shouldRepaint(covariant StereoWaveformPainter oldDelegate) {
    return oldDelegate.samples != samples;
  }
}