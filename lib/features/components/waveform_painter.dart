import 'dart:typed_data';
import 'dart:ui';
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
    if (samples.isEmpty || size.width <= 0) return;

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

    final int totalDataBins = samples.length ~/ 4;
    final double pixels = size.width;
    
    // Total columns to draw
    final int drawSteps = pixels.ceil();
    final double binsPerPixel = totalDataBins / pixels;

    // Height calculations
    final channelHeight = size.height / 2;
    final halfChannelHeight = channelHeight / 2;
    final leftCenterY = halfChannelHeight;
    final rightCenterY = channelHeight + halfChannelHeight;

    // OPTIMIZATION 1: Use Float32List for batch drawing (GPU friendly)
    // We need 2 lines per pixel column (Left channel line, Right channel line)
    // Each line has 2 points (Start, End)
    // Each point has 2 coordinates (x, y)
    // Total: drawSteps * 2 lines * 2 points * 2 coords = drawSteps * 8
    final rawPoints = Float32List(drawSteps * 8);
    int ptr = 0; // Pointer for insertion

    for (int i = 0; i < drawSteps; i++) {
      final int startBin = (i * binsPerPixel).floor();
      final int endBin = ((i + 1) * binsPerPixel).ceil();

      // Clamp
      final int actualStart = startBin.clamp(0, totalDataBins);
      final int actualEnd = endBin.clamp(0, totalDataBins);

      if (actualStart >= actualEnd) continue;

      // OPTIMIZATION 2: Adaptive Sampling (Striding)
      // If a single pixel covers 10,000 samples, we don't need to check all 10,000.
      // Checking 50-100 spread out samples is visually identical on a 1px wide line.
      int step = 1;
      final int range = actualEnd - actualStart;
      if (range > 100) {
        step = (range / 100).ceil();
      }

      double lMin = 1.0;
      double lMax = -1.0;
      double rMin = 1.0;
      double rMax = -1.0;

      // Inner Loop with Step
      for (int j = actualStart; j < actualEnd; j += step) {
        final int sampleIdx = j * 4;
        if (sampleIdx + 3 >= samples.length) break;

        final v0 = samples[sampleIdx];     // L_min
        final v1 = samples[sampleIdx + 1]; // L_max
        final v2 = samples[sampleIdx + 2]; // R_min
        final v3 = samples[sampleIdx + 3]; // R_max

        if (v0 < lMin) lMin = v0;
        if (v1 > lMax) lMax = v1;
        if (v2 < rMin) rMin = v2;
        if (v3 > rMax) rMax = v3;
      }

      if (lMax < lMin) continue;

      final double x = i.toDouble();

      // --- Left Channel Line ---
      // P1 (Top)
      rawPoints[ptr++] = x; 
      rawPoints[ptr++] = leftCenterY - (lMax * halfChannelHeight);
      // P2 (Bottom)
      rawPoints[ptr++] = x;
      rawPoints[ptr++] = leftCenterY - (lMin * halfChannelHeight);

      // --- Right Channel Line ---
      // P1 (Top)
      rawPoints[ptr++] = x;
      rawPoints[ptr++] = rightCenterY - (rMax * halfChannelHeight);
      // P2 (Bottom)
      rawPoints[ptr++] = x;
      rawPoints[ptr++] = rightCenterY - (rMin * halfChannelHeight);
    }

    // OPTIMIZATION 3: Draw all points in one GPU call
    // This is significantly faster than path.moveTo / path.lineTo loops
    canvas.drawRawPoints(PointMode.lines, rawPoints, paint);
  }

  @override
  bool shouldRepaint(covariant StereoWaveformPainter oldDelegate) {
    // Only repaint if the data actually changed
    return oldDelegate.samples != samples || oldDelegate.color != color;
  }
}

class MonoWaveformPainter extends CustomPainter {
  /// Format per bin: [Left_Min, Left_Max, Right_Min, Right_Max]
  final List<double> samples;
  final Color color;
  final double strokeWidth;

  MonoWaveformPainter({
    required this.samples,
    required this.color,
    this.strokeWidth = 1.0,
  });

  @override
  void paint(Canvas canvas, Size size) {
    if (samples.isEmpty || size.width <= 0) return;

    final paint = Paint()
      ..color = color
      ..strokeWidth = strokeWidth
      ..style = PaintingStyle.stroke
      ..strokeCap = StrokeCap.round;

    final double width = size.width;
    final double halfHeight = size.height / 2;

    // 1. Calculate Total Bins
    //    Since each bin has 4 values [L_min, L_max, R_min, R_max]
    final int totalBins = samples.length ~/ 4;

    // 2. Prepare Batch Drawing Buffer
    //    1 vertical line per pixel column. 2 points per line. 2 coords per point.
    //    Total = width * 4
    final int pointsCount = width.ceil() * 4;
    final Float32List rawPoints = Float32List(pointsCount);
    int ptr = 0;

    // 3. Coordinate Mapping
    final double binsPerPixel = totalBins / width;

    // 4. OPTIMIZATION: Adaptive Step
    //    If we are zoomed out so far that 1 pixel covers 500 bins, 
    //    we don't need to check all 500. Checking ~50 is enough for a summary.
    int step = 1;
    if (binsPerPixel > 50) {
      step = (binsPerPixel / 50).ceil();
    }

    for (int x = 0; x < width; x++) {
      // Determine which bins fall into this pixel column
      final int startBin = (x * binsPerPixel).floor();
      final int endBin = ((x + 1) * binsPerPixel).ceil();

      final int actualStart = startBin.clamp(0, totalBins);
      final int actualEnd = endBin.clamp(0, totalBins);

      if (actualStart >= actualEnd) continue;

      // Track Min/Max for this pixel
      // We initialize with values that will definitely be overwritten
      double minAmp = 1.0; 
      double maxAmp = -1.0;
      bool hasData = false;

      // Inner Loop: Iterate BINS
      for (int i = actualStart; i < actualEnd; i += step) {
        final int idx = i * 4; // Jump 4 values at a time
        
        if (idx + 3 >= samples.length) break;

        // Read Pre-calculated Extremes
        final double lMin = samples[idx];     // Index 0
        final double lMax = samples[idx + 1]; // Index 1
        final double rMin = samples[idx + 2]; // Index 2
        final double rMax = samples[idx + 3]; // Index 3

        // MERGE STEREO TO MONO
        // We average the channels to get the "Mono Mix" level
        final double monoMin = (lMin + rMin) / 2.0;
        final double monoMax = (lMax + rMax) / 2.0;

        if (!hasData) {
            minAmp = monoMin;
            maxAmp = monoMax;
            hasData = true;
        } else {
            // We want the visual peak-to-peak range for this pixel
            if (monoMin < minAmp) minAmp = monoMin;
            if (monoMax > maxAmp) maxAmp = monoMax;
        }
      }

      if (!hasData) continue;

      // Ensure we draw at least a dot if the wave is silent/flat
      if (maxAmp == minAmp) {
        maxAmp += 0.01;
        minAmp -= 0.01;
      }

      final double xPos = x.toDouble();

      // Top Point (Max Amplitude) - Invert Y
      rawPoints[ptr++] = xPos;
      rawPoints[ptr++] = halfHeight - (maxAmp * halfHeight);

      // Bottom Point (Min Amplitude) - Invert Y
      rawPoints[ptr++] = xPos;
      rawPoints[ptr++] = halfHeight - (minAmp * halfHeight);
    }

    // Single GPU Draw Call
    canvas.drawRawPoints(PointMode.lines, rawPoints, paint);
  }

  @override
  bool shouldRepaint(covariant MonoWaveformPainter oldDelegate) {
    return oldDelegate.samples != samples || 
           oldDelegate.color != color;
  }
}

class StereoWaveformClipPainter extends CustomPainter {
  /// Expecting interleaved samples: [L, R, L, R, ...]
  final List<double> samples;
  final Color color;
  final double strokeWidth;

  StereoWaveformClipPainter({
    required this.samples,
    required this.color,
    this.strokeWidth = 1.0,
  });

  @override
  void paint(Canvas canvas, Size size) {
    if (samples.isEmpty || size.width <= 0) return;

    final paint = Paint()
      ..color = color
      ..strokeWidth = strokeWidth
      ..style = PaintingStyle.stroke
      ..strokeCap = StrokeCap.round;

    // Draw a faint divider line between channels
    final dividerPaint = Paint()
      ..color = Colors.white.withAlpha(30)
      ..strokeWidth = 1.0;
    
    canvas.drawLine(
      Offset(0, size.height / 2),
      Offset(size.width, size.height / 2),
      dividerPaint,
    );

    // ---------------------------------------------------------
    // STEREO LOGIC: Interleaved means Total Frames = Length / 2
    // ---------------------------------------------------------
    final int totalFrames = samples.length ~/ 2;
    final double width = size.width;
    
    // We need 2 lines per pixel column (Left Line + Right Line)
    // Each line needs 2 points. Each point has (x, y).
    // Total floats = width * 2 lines * 2 points * 2 coords = width * 8
    final int pointsCount = width.ceil() * 8;
    final Float32List rawPoints = Float32List(pointsCount);
    int ptr = 0;

    final double framesPerPixel = totalFrames / width;
    
    // Height Math
    // Left Channel occupies top 50% (Center at 25%)
    // Right Channel occupies bottom 50% (Center at 75%)
    final double channelHeight = size.height / 2;
    final double halfChannelHeight = channelHeight / 2;
    final double leftCenterY = halfChannelHeight; 
    final double rightCenterY = size.height - halfChannelHeight; 

    // OPTIMIZATION: Adaptive Step
    // If 1 pixel covers 10,000 frames, only check ~100 of them.
    int step = 1;
    if (framesPerPixel > 100) {
      step = (framesPerPixel / 100).ceil();
    }

    for (int x = 0; x < width; x++) {
      final int startFrame = (x * framesPerPixel).floor();
      final int endFrame = ((x + 1) * framesPerPixel).ceil();

      final int actualStart = startFrame.clamp(0, totalFrames);
      final int actualEnd = endFrame.clamp(0, totalFrames);

      if (actualStart >= actualEnd) continue;

      // Min/Max for Left (L) and Right (R)
      double minL = 0.0, maxL = 0.0;
      double minR = 0.0, maxR = 0.0;

      // Inner Loop: iterate FRAMES (pairs of samples)
      for (int i = actualStart; i < actualEnd; i += step) {
        final int sampleIdx = i * 2; // Interleaved index
        
        // Safety check just in case
        if (sampleIdx + 1 >= samples.length) break;

        final double lVal = samples[sampleIdx];
        final double rVal = samples[sampleIdx + 1];

        if (lVal < minL) minL = lVal;
        if (lVal > maxL) maxL = lVal;

        if (rVal < minR) minR = rVal;
        if (rVal > maxR) maxR = rVal;
      }

      // Visual tweak: ensure we always draw at least a dot if values are flat
      if (maxL == minL) { maxL += 0.01; minL -= 0.01; }
      if (maxR == minR) { maxR += 0.01; minR -= 0.01; }

      final double xPos = x.toDouble();

      // --- LEFT CHANNEL (Top) ---
      // Invert Y: Positive samples go UP (subtract from center)
      rawPoints[ptr++] = xPos;
      rawPoints[ptr++] = leftCenterY - (maxL * halfChannelHeight); // Top point
      rawPoints[ptr++] = xPos;
      rawPoints[ptr++] = leftCenterY - (minL * halfChannelHeight); // Bottom point

      // --- RIGHT CHANNEL (Bottom) ---
      rawPoints[ptr++] = xPos;
      rawPoints[ptr++] = rightCenterY - (maxR * halfChannelHeight); // Top point
      rawPoints[ptr++] = xPos;
      rawPoints[ptr++] = rightCenterY - (minR * halfChannelHeight); // Bottom point
    }

    // Single Batch Draw Call
    canvas.drawRawPoints(PointMode.lines, rawPoints, paint);
  }

  @override
  bool shouldRepaint(covariant StereoWaveformClipPainter oldDelegate) {
    return oldDelegate.samples != samples || 
           oldDelegate.color != color;
  }
}