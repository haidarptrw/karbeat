import 'dart:typed_data';
import 'dart:ui';
import 'package:flutter/material.dart';

import 'dart:ffi' as ffi;

import 'package:karbeat/src/rust/api/waveform.dart'; 

/// Create Float32List of waveform buffer 
/// from raw pointer to the waveform buffer at the Rust memory
Float32List createZeroCopyWaveformView(WaveformHandle handle) {
  final ptrAddress = handle.getPointer();
  final len = handle.getLen();
  
  if (ptrAddress == 0 || len == 0) {
    return Float32List(0);
  }
  
  // Cast the raw integer address back into a C-Pointer
  final ptr = ffi.Pointer<ffi.Float>.fromAddress(ptrAddress);
  
  // Create a Dart Float32List that looks directly at the Rust memory!
  return ptr.asTypedList(len);
}

// =============================================================================
// 1. StereoWaveformPainter  (Overview — Audio Properties screen)
// =============================================================================

class StereoWaveformPainter extends CustomPainter {
  /// Zero-copy Float32List interleaved stereo samples: [L, R, L, R, ...]
  final Float32List samples;
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

        // REMOVED `* _kI8Scale` - we are using raw f32 floats now!
        final double l = samples[sampleIdx];
        final double r = samples[sampleIdx + 1];

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
// 3. StereoWaveformClipPainter  (Timeline Clips — main optimization target)
// =============================================================================

class StereoWaveformClipPainter extends CustomPainter {
  final Float32List samples;
  final Color color;
  final double strokeWidth;
  final double zoomLevel;
  final double offsetTicks;
  final double samplesPerTick;
  final ScrollController scrollController;
  final double clipLeftOffset;

  StereoWaveformClipPainter({
    required this.samples,
    required this.color,
    this.strokeWidth = 1.0,
    required this.zoomLevel,
    required this.offsetTicks,
    required this.samplesPerTick,
    required this.scrollController,
    required this.clipLeftOffset,
  }) : super(repaint: scrollController);

  @override
  void paint(Canvas canvas, Size size) {
    if (samples.isEmpty || size.width <= 0) return;

    final int totalFrames = samples.length ~/ 2;
    final double framesPerPixel = zoomLevel * samplesPerTick;

    final paint = Paint()
      ..color = color
      ..strokeWidth = strokeWidth
      ..style = PaintingStyle.stroke
      ..strokeCap = StrokeCap.butt;

    double scrollOffset = 0;
    double viewportWidth = 2000;
    if (scrollController.hasClients) {
      scrollOffset = scrollController.offset;
      if (scrollController.position.hasViewportDimension) {
        viewportWidth = scrollController.position.viewportDimension;
      }
    }
    const double pad = 50;
    final double vLeft = (scrollOffset - clipLeftOffset - pad).clamp(0, size.width);
    final double vRight = (scrollOffset - clipLeftOffset + viewportWidth + pad).clamp(0, size.width);

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
      _paintConnectedLines(canvas, size, paint, totalFrames, vLeft, vRight, scrollOffset, viewportWidth, pad);
    } else {
      _paintVerticalBars(canvas, size, paint, totalFrames, framesPerPixel, vLeft, vRight);
    }
  }

  void _paintConnectedLines(
    Canvas canvas, Size size, Paint paint, int totalFrames,
    double vLeft, double vRight, double scrollOffset, double viewportWidth, double pad,
  ) {
    final double startFrameFloat = (offsetTicks + (vLeft * zoomLevel)) * samplesPerTick;
    final double endFrameFloat = (offsetTicks + (vRight * zoomLevel)) * samplesPerTick;

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
    final double invSamplesPerTick = 1.0 / samplesPerTick;

    int ptr = 0;

    for (int i = startFrame; i < endFrame; i++) {
      final double x = ((i * invSamplesPerTick) - offsetTicks) * invZoom;

      final int idx = i * 2;
      // 2. Removed `* _kI8Scale`
      final double l = samples[idx]; 
      final double r = samples[idx + 1];

      leftPoints[ptr] = x;
      leftPoints[ptr + 1] = leftCenterY - (l * quarterHeight);
      rightPoints[ptr] = x;
      rightPoints[ptr + 1] = rightCenterY - (r * quarterHeight);
      ptr += 2;
    }

    canvas.drawRawPoints(PointMode.polygon, leftPoints, paint);
    canvas.drawRawPoints(PointMode.polygon, rightPoints, paint);
  }

  void _paintVerticalBars(
    Canvas canvas, Size size, Paint paint, int totalFrames,
    double framesPerPixel, double vLeft, double vRight,
  ) {
    final double dataFirstPixel = -offsetTicks / zoomLevel;
    final double dataLastPixel = ((totalFrames / samplesPerTick) - offsetTicks) / zoomLevel;

    final int startPixel = vLeft.floor().clamp(dataFirstPixel.floor(), dataLastPixel.ceil()).clamp(0, size.width.ceil());
    final int endPixel = vRight.ceil().clamp(dataFirstPixel.floor(), dataLastPixel.ceil()).clamp(0, size.width.ceil());

    if (startPixel >= endPixel) return;

    final int pixelCount = endPixel - startPixel;
    final Float32List rawPoints = Float32List(pixelCount * 8);
    int ptr = 0;

    final double quarterHeight = size.height / 4;
    final double leftCenterY = quarterHeight;
    final double rightCenterY = size.height * 0.75;

    int step = 1;
    if (framesPerPixel > 50) step = (framesPerPixel / 50).ceil();

    for (int x = startPixel; x < endPixel; x++) {
      final double pixelTimelinePos = offsetTicks + (x * zoomLevel);
      final double sourcePos = pixelTimelinePos * samplesPerTick;

      final int startFrame = sourcePos.floor();
      final int endFrame = (sourcePos + framesPerPixel).ceil();
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
        final int idx = i * 2;
        if (idx + 1 >= samples.length) break;

        // 3. Removed `* _kI8Scale`
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

      if (lMax == lMin) { lMax += 0.01; lMin -= 0.01; }
      if (rMax == rMin) { rMax += 0.01; rMin -= 0.01; }

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

    canvas.drawRawPoints(
      PointMode.lines,
      Float32List.sublistView(rawPoints, 0, ptr),
      paint,
    );
  }

  @override
  bool shouldRepaint(covariant StereoWaveformClipPainter oldDelegate) {
    return oldDelegate.offsetTicks != offsetTicks ||
        oldDelegate.zoomLevel != zoomLevel ||
        oldDelegate.samplesPerTick != samplesPerTick ||
        oldDelegate.scrollController != scrollController ||
        oldDelegate.clipLeftOffset != clipLeftOffset ||
        !identical(oldDelegate.samples, samples) ||
        oldDelegate.color != color;
  }
}