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
  /// Raw interleaved stereo samples: [L, R, L, R, ...]
  final List<double> samples;
  final Color color;
  final double strokeWidth;

  // CONTEXT PARAMETERS
  final double zoomLevel;     // Samples per Pixel (Timeline)
  final double offsetSamples; // Scroll Offset (Timeline domain)
  final double ratio;         // sourceRate / projectRate

  StereoWaveformClipPainter({
    required this.samples,
    required this.color,
    this.strokeWidth = 1.0,
    required this.zoomLevel,
    required this.offsetSamples,
    this.ratio = 1.0, 
  });

  @override
  void paint(Canvas canvas, Size size) {
    if (samples.isEmpty || size.width <= 0) return;

    final paint = Paint()
      ..color = color
      ..strokeWidth = strokeWidth
      ..style = PaintingStyle.stroke
      ..strokeCap = StrokeCap.round;

    // Draw Divider Line (Center)
    final dividerPaint = Paint()
      ..color = Colors.white.withOpacity(0.2)
      ..strokeWidth = 1.0;
    canvas.drawLine(
      Offset(0, size.height / 2),
      Offset(size.width, size.height / 2),
      dividerPaint,
    );

    // 1. Setup Metrics
    final int totalFrames = samples.length ~/ 2;
    final double framesPerPixel = zoomLevel * ratio;
    
    // Layout centers
    final double halfHeight = size.height / 2;
    final double quarterHeight = halfHeight / 2;
    final double leftCenterY = quarterHeight; 
    final double rightCenterY = halfHeight + quarterHeight;

    // 2. DECISION: High Detail (Polyline) vs Low Detail (Peak Bars)
    // If 1 pixel covers fewer than 3 frames, we draw continuous lines (curvy).
    // Otherwise, we draw vertical peak bars (traditional waveform).
    if (framesPerPixel < 3.0) {
      _drawPolylineWaveform(
        canvas: canvas,
        width: size.width,
        totalFrames: totalFrames,
        leftCenterY: leftCenterY,
        rightCenterY: rightCenterY,
        ampHeight: quarterHeight,
        paint: paint,
      );
    } else {
      _drawPeakWaveform(
        canvas: canvas,
        width: size.width,
        totalFrames: totalFrames,
        framesPerPixel: framesPerPixel,
        leftCenterY: leftCenterY,
        rightCenterY: rightCenterY,
        ampHeight: quarterHeight,
        paint: paint,
      );
    }
  }

  // --- MODE A: ZOOMED OUT (Peak Bars) ---
  void _drawPeakWaveform({
    required Canvas canvas,
    required double width,
    required int totalFrames,
    required double framesPerPixel,
    required double leftCenterY,
    required double rightCenterY,
    required double ampHeight,
    required Paint paint,
  }) {
    // We need 4 points per pixel column (L_top, L_btm, R_top, R_btm) -> 8 floats
    // But since drawRawPoints(lines) takes pairs, we strictly need:
    // Line L: (x, y1) -> (x, y2)
    // Line R: (x, y3) -> (x, y4)
    final int pointsCount = width.ceil() * 8;
    final Float32List rawPoints = Float32List(pointsCount);
    int ptr = 0;

    // Optimization: stride
    int step = 1;
    if (framesPerPixel > 200) {
      step = (framesPerPixel / 200).ceil();
    }

    for (int x = 0; x < width; x++) {
      // Calculate Timeline Window
      final double pixelTimelinePos = offsetSamples + (x * zoomLevel);
      final double sourcePos = pixelTimelinePos * ratio;

      final int startFrame = sourcePos.floor();
      final int endFrame = (sourcePos + framesPerPixel).ceil();

      final int actualStart = startFrame.clamp(0, totalFrames);
      final int actualEnd = endFrame.clamp(0, totalFrames);

      // If out of bounds (gap/silence), skip drawing
      if (actualStart >= actualEnd) {
        ptr += 8; 
        continue;
      }

      // Find Min/Max
      double lMin = 1.0, lMax = -1.0;
      double rMin = 1.0, rMax = -1.0;
      bool hasData = false;

      for (int i = actualStart; i < actualEnd; i += step) {
        final int idx = i * 2;
        if (idx + 1 >= samples.length) break;

        final double l = samples[idx];
        final double r = samples[idx + 1];

        if (!hasData) {
          lMin = l; lMax = l;
          rMin = r; rMax = r;
          hasData = true;
        } else {
          if (l < lMin) lMin = l;
          if (l > lMax) lMax = l;
          if (r < rMin) rMin = r;
          if (r > rMax) rMax = r;
        }
      }

      if (!hasData) {
        ptr += 8; 
        continue;
      }

      // Visual tweak for flatness
      if (lMax == lMin) { lMax += 0.01; lMin -= 0.01; }
      if (rMax == rMin) { rMax += 0.01; rMin -= 0.01; }

      final double xPos = x.toDouble();

      // LEFT CHANNEL (Top Half)
      // Invert Y: Up is negative relative to center
      rawPoints[ptr++] = xPos;
      rawPoints[ptr++] = leftCenterY - (lMax * ampHeight);
      rawPoints[ptr++] = xPos;
      rawPoints[ptr++] = leftCenterY - (lMin * ampHeight);

      // RIGHT CHANNEL (Bottom Half)
      rawPoints[ptr++] = xPos;
      rawPoints[ptr++] = rightCenterY - (rMax * ampHeight);
      rawPoints[ptr++] = xPos;
      rawPoints[ptr++] = rightCenterY - (rMin * ampHeight);
    }

    canvas.drawRawPoints(PointMode.lines, rawPoints, paint);
  }

  // --- MODE B: ZOOMED IN (Polyline/Curve) ---
  void _drawPolylineWaveform({
    required Canvas canvas,
    required double width,
    required int totalFrames,
    required double leftCenterY,
    required double rightCenterY,
    required double ampHeight,
    required Paint paint,
  }) {
    // For polyline, we calculate 1 point per pixel.
    // Float32List is slightly harder for Polylines with breaks, 
    // but for a clip we assume continuous data unless end of file.
    
    // We will build two paths or lists of points. 
    // Float32List is faster than Path.
    
    final List<Offset> leftPoints = [];
    final List<Offset> rightPoints = [];

    for (int x = 0; x < width; x++) {
      final double pixelTimelinePos = offsetSamples + (x * zoomLevel);
      final double sourcePos = pixelTimelinePos * ratio;

      // Bounds Check
      if (sourcePos < 0 || sourcePos >= totalFrames - 1) {
        // Stop drawing if we hit the end of the file
        if (sourcePos >= totalFrames) break;
      }

      // Linear Interpolation for Smooth Curves
      final int idx = sourcePos.floor();
      final double t = sourcePos - idx; // Fractional part

      final int basePtr = idx * 2;
      if (basePtr + 3 >= samples.length) break;

      // Get current and next frame
      final double l1 = samples[basePtr];
      final double r1 = samples[basePtr + 1];
      final double l2 = samples[basePtr + 2];
      final double r2 = samples[basePtr + 3];

      // Lerp
      final double lVal = l1 + (l2 - l1) * t;
      final double rVal = r1 + (r2 - r1) * t;

      final double xPos = x.toDouble();

      leftPoints.add(Offset(xPos, leftCenterY - (lVal * ampHeight)));
      rightPoints.add(Offset(xPos, rightCenterY - (rVal * ampHeight)));
    }

    if (leftPoints.isNotEmpty) {
      canvas.drawPoints(PointMode.polygon, leftPoints, paint);
      canvas.drawPoints(PointMode.polygon, rightPoints, paint);
    }
  }

  @override
  bool shouldRepaint(covariant StereoWaveformClipPainter oldDelegate) {
    return oldDelegate.offsetSamples != offsetSamples || 
           oldDelegate.zoomLevel != zoomLevel ||
           oldDelegate.samples != samples || 
           oldDelegate.color != color;
  }
}

class MonoWaveformClipPainter extends CustomPainter {
  /// Raw interleaved stereo samples: [L, R, L, R, ...]
  final List<double> samples;
  final Color color;
  final double strokeWidth;
  
  // CONTEXT PARAMETERS
  final double zoomLevel;     // Samples per Pixel (Timeline)
  final double offsetSamples; // Scroll Offset (Timeline domain)
  final double ratio;         // sourceRate / projectRate

  MonoWaveformClipPainter({
    required this.samples,
    required this.color,
    this.strokeWidth = 1.0,
    required this.zoomLevel,
    required this.offsetSamples,
    this.ratio = 1.0, 
  });

  @override
  void paint(Canvas canvas, Size size) {
    // Safety check: Needs at least 2 samples (1 frame) to draw anything
    if (samples.length < 2 || size.width <= 0) return;

    final paint = Paint()
      ..color = color
      ..strokeWidth = strokeWidth
      ..style = PaintingStyle.stroke
      ..strokeCap = StrokeCap.round;

    final double width = size.width;
    final double halfHeight = size.height / 2;

    // 1. Calculate Total Audio FRAMES
    //    Stereo interleaved means Frame Count = Array Length / 2
    final int totalFrames = samples.length ~/ 2;

    // 2. Prepare Batch Drawing Buffer
    //    1 vertical line per pixel column -> 2 points (top, bottom) -> 4 floats (x,y,x,y)
    final int pointsCount = width.ceil() * 4;
    final Float32List rawPoints = Float32List(pointsCount);
    int ptr = 0;

    // 3. OPTIMIZATION: Adaptive Step
    //    How many SOURCE FRAMES correspond to ONE PIXEL?
    final double framesPerPixel = zoomLevel * ratio;

    //    If zoomed out (1 pixel = 10k frames), skip checking every single frame to save CPU.
    int step = 1;
    if (framesPerPixel > 200) {
      step = (framesPerPixel / 200).ceil();
    }

    // 4. DRAWING LOOP
    for (int x = 0; x < width; x++) {
      // A. Calculate Timeline Position
      final double pixelTimelinePos = offsetSamples + (x * zoomLevel);
      
      // B. Convert to Source Frame Index
      final double sourcePos = pixelTimelinePos * ratio;

      final int startFrame = sourcePos.floor();
      final int endFrame = (sourcePos + framesPerPixel).ceil();

      // C. Bounds Check (Frame Domain)
      final int actualStart = startFrame.clamp(0, totalFrames);
      final int actualEnd = endFrame.clamp(0, totalFrames);

      // Skip if out of data
      if (actualStart >= actualEnd) {
         ptr += 4; // Advance pointer to keep alignment
         continue; 
      }

      // D. Find Min/Max Amplitude
      double minAmp = 1.0; 
      double maxAmp = -1.0;
      bool hasData = false;

      // Inner Loop: Iterate FRAMES
      for (int i = actualStart; i < actualEnd; i += step) {
        final int sampleIdx = i * 2; // Jump by 2 for stereo
        
        // Safety: Ensure we have both L and R
        if (sampleIdx + 1 >= samples.length) break;

        final double lVal = samples[sampleIdx];
        final double rVal = samples[sampleIdx + 1];

        // E. Mono Merge: Average L+R
        final double monoVal = (lVal + rVal) / 2.0;

        if (!hasData) {
            minAmp = monoVal;
            maxAmp = monoVal;
            hasData = true;
        } else {
            if (monoVal < minAmp) minAmp = monoVal;
            if (monoVal > maxAmp) maxAmp = monoVal;
        }
      }

      if (!hasData) {
         ptr += 4;
         continue;
      }

      // Visual tweak: ensure we always draw at least a dot
      if (maxAmp == minAmp) {
        maxAmp += 0.01;
        minAmp -= 0.01;
      }

      final double xPos = x.toDouble();

      // Top Point (Inverted Y)
      rawPoints[ptr++] = xPos;
      rawPoints[ptr++] = halfHeight - (maxAmp * halfHeight);

      // Bottom Point (Inverted Y)
      rawPoints[ptr++] = xPos;
      rawPoints[ptr++] = halfHeight - (minAmp * halfHeight);
    }

    canvas.drawRawPoints(PointMode.lines, rawPoints, paint);
  }

  @override
  bool shouldRepaint(covariant MonoWaveformClipPainter oldDelegate) {
    return oldDelegate.offsetSamples != offsetSamples || 
           oldDelegate.zoomLevel != zoomLevel ||
           oldDelegate.samples != samples || 
           oldDelegate.color != color;
  }
}