import 'dart:typed_data';
import 'dart:ui';
import 'package:flutter/material.dart';

// =============================================================================
// Common constant for i8 → float conversion
// =============================================================================
const double _kI8Scale = 1.0 / 127.0;

// =============================================================================
// 1. StereoWaveformPainter  (Overview — Audio Properties screen)
// =============================================================================

class StereoWaveformPainter extends CustomPainter {
  /// Quantized i8 interleaved stereo samples: [L, R, L, R, ...]
  final Int8List samples;
  final Color color;
  final double strokeWidth;

  const StereoWaveformPainter({
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
      ..strokeCap = StrokeCap.butt;

    // Draw Divider Line (Center)
    final dividerPaint = Paint()
      ..color = Colors.white.withAlpha((0.2 * 255).round())
      ..strokeWidth = 1.0;

    canvas.drawLine(
      Offset(0, size.height / 2),
      Offset(size.width, size.height / 2),
      dividerPaint,
    );

    final int totalFrames = samples.length ~/ 2;
    final double width = size.width;
    final int pointsCount = width.ceil() * 8;
    final Float32List rawPoints = Float32List(pointsCount);
    int ptr = 0;

    final double framesPerPixel = totalFrames / width;

    final double halfHeight = size.height / 2;
    final double quarterHeight = halfHeight / 2;
    final double leftCenterY = quarterHeight;
    final double rightCenterY = halfHeight + quarterHeight;

    int step = 1;
    if (framesPerPixel > 50) {
      step = (framesPerPixel / 50).ceil();
    }

    for (int x = 0; x < width; x++) {
      final int startFrame = (x * framesPerPixel).floor();
      final int endFrame = ((x + 1) * framesPerPixel).ceil();

      final int actualStart = startFrame.clamp(0, totalFrames);
      final int actualEnd = endFrame.clamp(0, totalFrames);

      if (actualStart >= actualEnd) {
        ptr += 8;
        continue;
      }

      double lMin = 1.0, lMax = -1.0;
      double rMin = 1.0, rMax = -1.0;
      bool hasData = false;

      for (int i = actualStart; i < actualEnd; i += step) {
        final int sampleIdx = i * 2;
        if (sampleIdx + 1 >= samples.length) break;

        final double l = samples[sampleIdx] * _kI8Scale;
        final double r = samples[sampleIdx + 1] * _kI8Scale;

        if (!hasData) {
          lMin = l;
          lMax = l;
          rMin = r;
          rMax = r;
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

      if (lMax == lMin) {
        lMax += 0.01;
        lMin -= 0.01;
      }
      if (rMax == rMin) {
        rMax += 0.01;
        rMin -= 0.01;
      }

      final double xPos = x.toDouble();

      rawPoints[ptr++] = xPos;
      rawPoints[ptr++] = leftCenterY - (lMax * quarterHeight);
      rawPoints[ptr++] = xPos;
      rawPoints[ptr++] = leftCenterY - (lMin * quarterHeight);

      rawPoints[ptr++] = xPos;
      rawPoints[ptr++] = rightCenterY - (rMax * quarterHeight);
      rawPoints[ptr++] = xPos;
      rawPoints[ptr++] = rightCenterY - (rMin * quarterHeight);
    }

    canvas.drawRawPoints(PointMode.lines, rawPoints, paint);
  }

  @override
  bool shouldRepaint(covariant StereoWaveformPainter oldDelegate) {
    return !identical(oldDelegate.samples, samples) ||
        oldDelegate.color != color;
  }
}

// =============================================================================
// 2. MonoWaveformPainter  (Overview — pre-binned data)
// =============================================================================

class MonoWaveformPainter extends CustomPainter {
  /// Format per bin: [Left_Min, Left_Max, Right_Min, Right_Max] as i8
  final Int8List samples;
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
      ..strokeCap = StrokeCap.butt;

    final double width = size.width;
    final double halfHeight = size.height / 2;

    final int totalBins = samples.length ~/ 4;

    final int pointsCount = width.ceil() * 4;
    final Float32List rawPoints = Float32List(pointsCount);
    int ptr = 0;

    final double binsPerPixel = totalBins / width;

    int step = 1;
    if (binsPerPixel > 50) {
      step = (binsPerPixel / 50).ceil();
    }

    for (int x = 0; x < width; x++) {
      final int startBin = (x * binsPerPixel).floor();
      final int endBin = ((x + 1) * binsPerPixel).ceil();

      final int actualStart = startBin.clamp(0, totalBins);
      final int actualEnd = endBin.clamp(0, totalBins);

      if (actualStart >= actualEnd) continue;

      double minAmp = 1.0;
      double maxAmp = -1.0;
      bool hasData = false;

      for (int i = actualStart; i < actualEnd; i += step) {
        final int idx = i * 4;
        if (idx + 3 >= samples.length) break;

        final double lMin = samples[idx] * _kI8Scale;
        final double lMax = samples[idx + 1] * _kI8Scale;
        final double rMin = samples[idx + 2] * _kI8Scale;
        final double rMax = samples[idx + 3] * _kI8Scale;

        final double monoMin = (lMin + rMin) / 2.0;
        final double monoMax = (lMax + rMax) / 2.0;

        if (!hasData) {
          minAmp = monoMin;
          maxAmp = monoMax;
          hasData = true;
        } else {
          if (monoMin < minAmp) minAmp = monoMin;
          if (monoMax > maxAmp) maxAmp = monoMax;
        }
      }

      if (!hasData) continue;

      if (maxAmp == minAmp) {
        maxAmp += 0.01;
        minAmp -= 0.01;
      }

      final double xPos = x.toDouble();

      rawPoints[ptr++] = xPos;
      rawPoints[ptr++] = halfHeight - (maxAmp * halfHeight);

      rawPoints[ptr++] = xPos;
      rawPoints[ptr++] = halfHeight - (minAmp * halfHeight);
    }

    canvas.drawRawPoints(PointMode.lines, rawPoints, paint);
  }

  @override
  bool shouldRepaint(covariant MonoWaveformPainter oldDelegate) {
    return !identical(oldDelegate.samples, samples) ||
        oldDelegate.color != color;
  }
}

// =============================================================================
// 3. StereoWaveformClipPainter  (Timeline Clips — main optimization target)
// =============================================================================

class StereoWaveformClipPainter extends CustomPainter {
  final Int8List samples;
  final Color color;
  final double strokeWidth;
  final double zoomLevel;
  final double offsetSamples;
  final double ratio;

  /// Scroll controller used for repainting
  final ScrollController scrollController;
  final double clipLeftOffset;

  StereoWaveformClipPainter({
    required this.samples,
    required this.color,
    this.strokeWidth = 1.0,
    required this.zoomLevel,
    required this.offsetSamples,
    this.ratio = 1.0,
    required this.scrollController,
    required this.clipLeftOffset,
  }) : super(repaint: scrollController);

  @override
  void paint(Canvas canvas, Size size) {
    if (samples.isEmpty || size.width <= 0) return;

    final int totalFrames = samples.length ~/ 2;
    final double framesPerPixel = zoomLevel * ratio;

    final paint = Paint()
      ..color = color
      ..strokeWidth = strokeWidth
      ..style = PaintingStyle.stroke
      ..strokeCap = StrokeCap.butt;

    double scrollOffset = 0;
    double viewportWidth = 2000; // fallback
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

    // Center divider — only draw within visible range
    final double divLeft = vLeft.clamp(0, size.width);
    final double divRight = vRight.clamp(0, size.width);
    final dividerPaint = Paint()
      ..color = Colors.white.withAlpha(50)
      ..strokeWidth = 1.0;
    canvas.drawLine(
      Offset(divLeft, size.height / 2),
      Offset(divRight, size.height / 2),
      dividerPaint,
    );

    if (framesPerPixel < 1.0) {
      _paintConnectedLines(
        canvas,
        size,
        paint,
        totalFrames,
        vLeft,
        vRight,
        scrollOffset,
        viewportWidth,
        pad,
      );
    } else {
      _paintVerticalBars(
        canvas,
        size,
        paint,
        totalFrames,
        framesPerPixel,
        vLeft,
        vRight,
      );
    }
  }

  // OPTIMIZED FOR ZOOMED IN (Low Density)
  void _paintConnectedLines(
    Canvas canvas,
    Size size,
    Paint paint,
    int totalFrames,
    double vLeft,
    double vRight,
    double scrollOffset,
    double viewportWidth,
    double pad,
  ) {
    final double vLeft = (scrollOffset - clipLeftOffset - pad).clamp(
      0,
      size.width,
    );
    final double vRight = (scrollOffset - clipLeftOffset + viewportWidth + pad)
        .clamp(0, size.width);
    // Convert visible pixel range → frame range
    final double startFrameFloat =
        (offsetSamples + (vLeft * zoomLevel)) * ratio;
    final double endFrameFloat = (offsetSamples + (vRight * zoomLevel)) * ratio;

    final int startFrame = startFrameFloat.floor().clamp(0, totalFrames);
    final int endFrame = endFrameFloat.ceil().clamp(0, totalFrames);

    if (startFrame >= endFrame) return;

    final int frameCount = endFrame - startFrame;
    final Float32List leftPoints = Float32List(frameCount * 2);
    final Float32List rightPoints = Float32List(frameCount * 2);

    final double quarterHeight = size.height / 4;
    final double leftCenterY = quarterHeight;
    final double rightCenterY = size.height * 0.75;

    final double invZoom = 1.0 / zoomLevel;
    final double invRatio = 1.0 / ratio;

    int ptr = 0;

    for (int i = startFrame; i < endFrame; i++) {
      final double x = ((i * invRatio) - offsetSamples) * invZoom;

      final int idx = i * 2;
      final double l = samples[idx] * _kI8Scale;
      final double r = samples[idx + 1] * _kI8Scale;

      leftPoints[ptr] = x;
      leftPoints[ptr + 1] = leftCenterY - (l * quarterHeight);

      rightPoints[ptr] = x;
      rightPoints[ptr + 1] = rightCenterY - (r * quarterHeight);

      ptr += 2;
    }

    canvas.drawRawPoints(PointMode.polygon, leftPoints, paint);
    canvas.drawRawPoints(PointMode.polygon, rightPoints, paint);
  }

  // OPTIMIZED FOR ZOOMED OUT (High Density)
  void _paintVerticalBars(
    Canvas canvas,
    Size size,
    Paint paint,
    int totalFrames,
    double framesPerPixel,
    double vLeft,
    double vRight,
  ) {
    // Intersect data bounds with viewport bounds
    final double dataFirstPixel = -offsetSamples / zoomLevel;
    final double dataLastPixel =
        ((totalFrames / ratio) - offsetSamples) / zoomLevel;

    final int startPixel = vLeft
        .floor()
        .clamp(dataFirstPixel.floor(), dataLastPixel.ceil())
        .clamp(0, size.width.ceil());
    final int endPixel = vRight
        .ceil()
        .clamp(dataFirstPixel.floor(), dataLastPixel.ceil())
        .clamp(0, size.width.ceil());

    if (startPixel >= endPixel) return;

    final int pixelCount = endPixel - startPixel;
    final Float32List rawPoints = Float32List(
      pixelCount * 8,
    ); // 4 points (2 vertical lines) per pixel
    int ptr = 0;

    final double quarterHeight = size.height / 4;
    final double leftCenterY = quarterHeight;
    final double rightCenterY = size.height * 0.75;

    // Adaptive step for massive zoom-outs
    int step = 1;
    if (framesPerPixel > 50) step = (framesPerPixel / 50).ceil();

    for (int x = startPixel; x < endPixel; x++) {
      final double pixelTimelinePos = offsetSamples + (x * zoomLevel);
      final double sourcePos = pixelTimelinePos * ratio;

      final int startFrame = sourcePos.floor();
      final int endFrame = (sourcePos + framesPerPixel).ceil();
      final int actualStart = startFrame.clamp(0, totalFrames);
      final int actualEnd = endFrame.clamp(0, totalFrames);

      if (actualStart >= actualEnd) {
        ptr += 8; // Skip this pixel slot in buffer
        continue;
      }

      // Min/Max Search
      double lMin = 1.0, lMax = -1.0;
      double rMin = 1.0, rMax = -1.0;
      bool hasData = false;

      for (int i = actualStart; i < actualEnd; i += step) {
        final int idx = i * 2;
        if (idx + 1 >= samples.length) break;

        final double l = samples[idx] * _kI8Scale;
        final double r = samples[idx + 1] * _kI8Scale;

        if (!hasData) {
          lMin = l;
          lMax = l;
          rMin = r;
          rMax = r;
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

      // Ensure min/max are distinct so we draw at least a dot
      if (lMax == lMin) {
        lMax += 0.01;
        lMin -= 0.01;
      }
      if (rMax == rMin) {
        rMax += 0.01;
        rMin -= 0.01;
      }

      final double xPos = x.toDouble();

      // Left Channel Vertical Bar
      rawPoints[ptr++] = xPos;
      rawPoints[ptr++] = leftCenterY - (lMax * quarterHeight);
      rawPoints[ptr++] = xPos;
      rawPoints[ptr++] = leftCenterY - (lMin * quarterHeight);

      // Right Channel Vertical Bar
      rawPoints[ptr++] = xPos;
      rawPoints[ptr++] = rightCenterY - (rMax * quarterHeight);
      rawPoints[ptr++] = xPos;
      rawPoints[ptr++] = rightCenterY - (rMin * quarterHeight);
    }

    // Note: PointMode.lines draws unconnected segments (p0-p1, p2-p3)
    canvas.drawRawPoints(
      PointMode.lines,
      Float32List.sublistView(rawPoints, 0, ptr),
      paint,
    );
  }

  @override
  bool shouldRepaint(covariant StereoWaveformClipPainter oldDelegate) {
    return oldDelegate.offsetSamples != offsetSamples ||
        oldDelegate.zoomLevel != zoomLevel ||
        oldDelegate.ratio != ratio ||
        oldDelegate.scrollController != scrollController ||
        oldDelegate.clipLeftOffset != clipLeftOffset ||
        !identical(oldDelegate.samples, samples) ||
        oldDelegate.color != color;
  }
}
// =============================================================================
// 4. MonoWaveformClipPainter  (Timeline Clips — mono merge)
// =============================================================================

class MonoWaveformClipPainter extends CustomPainter {
  final Int8List samples;
  final Color color;
  final double strokeWidth;
  final double zoomLevel;
  final double offsetSamples;
  final double ratio;
  final ScrollController scrollController;
  final double clipLeftOffset;

  MonoWaveformClipPainter({
    required this.samples,
    required this.color,
    this.strokeWidth = 1.0,
    required this.zoomLevel,
    required this.offsetSamples,
    this.ratio = 1.0,
    required this.scrollController,
    required this.clipLeftOffset,
  }) : super(repaint: scrollController);

  @override
  void paint(Canvas canvas, Size size) {
    if (samples.length < 2 || size.width <= 0) return;

    final paint = Paint()
      ..color = color
      ..strokeWidth = strokeWidth
      ..style = PaintingStyle.stroke
      ..strokeCap = StrokeCap.butt;

    final int totalFrames = samples.length ~/ 2;
    final double framesPerPixel = zoomLevel * ratio;

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

    if (framesPerPixel < 1.0) {
      _paintConnectedLines(canvas, size, paint, totalFrames, vLeft, vRight);
    } else {
      _paintVerticalBars(
        canvas,
        size,
        paint,
        totalFrames,
        framesPerPixel,
        vLeft,
        vRight,
      );
    }
  }

  void _paintConnectedLines(
    Canvas canvas,
    Size size,
    Paint paint,
    int totalFrames,
    double vLeft,
    double vRight,
  ) {
    final double startFrameFloat =
        (offsetSamples + (vLeft * zoomLevel)) * ratio;
    final double endFrameFloat = (offsetSamples + (vRight * zoomLevel)) * ratio;

    final int startFrame = startFrameFloat.floor().clamp(0, totalFrames);
    final int endFrame = endFrameFloat.ceil().clamp(0, totalFrames);

    if (startFrame >= endFrame) return;

    final int frameCount = endFrame - startFrame;
    final Float32List points = Float32List(frameCount * 2);

    final double halfHeight = size.height / 2;
    final double invZoom = 1.0 / zoomLevel;
    final double invRatio = 1.0 / ratio;

    int ptr = 0;

    for (int i = startFrame; i < endFrame; i++) {
      final double x = ((i * invRatio) - offsetSamples) * invZoom;

      final int idx = i * 2;
      final double l = samples[idx] * _kI8Scale;
      final double r = samples[idx + 1] * _kI8Scale;
      final double monoVal = (l + r) / 2.0;

      points[ptr++] = x;
      points[ptr++] = halfHeight - (monoVal * halfHeight);
    }

    canvas.drawRawPoints(PointMode.polygon, points, paint);
  }

  void _paintVerticalBars(
    Canvas canvas,
    Size size,
    Paint paint,
    int totalFrames,
    double framesPerPixel,
    double vLeft,
    double vRight,
  ) {
    final double dataFirstPixel = -offsetSamples / zoomLevel;
    final double dataLastPixel =
        ((totalFrames / ratio) - offsetSamples) / zoomLevel;

    final int startPixel = vLeft
        .floor()
        .clamp(dataFirstPixel.floor(), dataLastPixel.ceil())
        .clamp(0, size.width.ceil());
    final int endPixel = vRight
        .ceil()
        .clamp(dataFirstPixel.floor(), dataLastPixel.ceil())
        .clamp(0, size.width.ceil());

    if (startPixel >= endPixel) return;

    final int pixelCount = endPixel - startPixel;
    final Float32List rawPoints = Float32List(pixelCount * 4);
    int ptr = 0;

    final double halfHeight = size.height / 2;
    int step = 1;
    if (framesPerPixel > 50) step = (framesPerPixel / 50).ceil();

    for (int x = startPixel; x < endPixel; x++) {
      final double pixelTimelinePos = offsetSamples + (x * zoomLevel);
      final double sourcePos = pixelTimelinePos * ratio;
      final int startFrame = sourcePos.floor();
      final int endFrame = (sourcePos + framesPerPixel).ceil();

      final int actualStart = startFrame.clamp(0, totalFrames);
      final int actualEnd = endFrame.clamp(0, totalFrames);

      if (actualStart >= actualEnd) {
        ptr += 4;
        continue;
      }

      double minAmp = 1.0;
      double maxAmp = -1.0;
      bool hasData = false;

      for (int i = actualStart; i < actualEnd; i += step) {
        final int idx = i * 2;
        if (idx + 1 >= samples.length) break;
        final double l = samples[idx] * _kI8Scale;
        final double r = samples[idx + 1] * _kI8Scale;
        final double monoVal = (l + r) / 2.0;

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

      if (maxAmp == minAmp) {
        maxAmp += 0.01;
        minAmp -= 0.01;
      }

      final double xPos = x.toDouble();
      rawPoints[ptr++] = xPos;
      rawPoints[ptr++] = halfHeight - (maxAmp * halfHeight);
      rawPoints[ptr++] = xPos;
      rawPoints[ptr++] = halfHeight - (minAmp * halfHeight);
    }

    canvas.drawRawPoints(PointMode.lines, rawPoints, paint);
  }

  @override
  bool shouldRepaint(covariant MonoWaveformClipPainter oldDelegate) {
    return oldDelegate.offsetSamples != offsetSamples ||
        oldDelegate.zoomLevel != zoomLevel ||
        oldDelegate.ratio != ratio ||
        oldDelegate.scrollController != scrollController ||
        oldDelegate.clipLeftOffset != clipLeftOffset ||
        !identical(oldDelegate.samples, samples) ||
        oldDelegate.color != color;
  }
}
