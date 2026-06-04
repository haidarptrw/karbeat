import 'dart:async';
import 'dart:collection';
import 'package:flutter/material.dart';
import 'package:karbeat/src/rust/api/monitor.dart'; // Your generated FFI

class DawPerformanceMonitor extends StatefulWidget {
  const DawPerformanceMonitor({super.key});

  @override
  State<DawPerformanceMonitor> createState() => _DawPerformanceMonitorState();
}

class _DawPerformanceMonitorState extends State<DawPerformanceMonitor> {
  static const int maxDataPoints = 100;

  // Rolling queues for the line graph
  final Queue<double> _dspHistory = Queue<double>();
  final Queue<double> _cpuHistory = Queue<double>();

  double _currentRamMb = 0.0;
  double _totalRamMb = 0.0;
  double _currentDsp = 0.0;

  StreamSubscription? _monitorSub;

  @override
  void initState() {
    super.initState();
    // Fill the queues with zeros initially
    for (int i = 0; i < maxDataPoints; i++) {
      _dspHistory.add(0.0);
      _cpuHistory.add(0.0);
    }

    _monitorSub = startPerformanceMonitor().listen((metrics) {
      setState(() {
        _currentRamMb = metrics.ramUsageMb;
        _totalRamMb = metrics.totalRamMb;
        _currentDsp = metrics.dspHeadroom;

        // Advance the rolling graph
        _dspHistory.removeFirst();
        _dspHistory.add(metrics.dspHeadroom.clamp(0.0, 100.0));

        _cpuHistory.removeFirst();
        _cpuHistory.add(metrics.osCpuUsage.clamp(0.0, 100.0));
      });
    });
  }

  @override
  void dispose() {
    _monitorSub?.cancel();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return AspectRatio(
      aspectRatio: 3.5, // Width will always be 3.5x the height
      child: Container(
        padding: const EdgeInsets.all(8),
        decoration: BoxDecoration(
          color: Colors.black87,
          borderRadius: BorderRadius.circular(4),
          border: Border.all(color: Colors.grey.shade800),
        ),
        child: Column(
          children: [
            FittedBox(
              fit: BoxFit.scaleDown,
              alignment: Alignment.centerLeft,
              child: Row(
                children: [
                  Text(
                    "DSP: ${_currentDsp.toStringAsFixed(1)}%",
                    style: TextStyle(
                      color: _currentDsp > 85 ? Colors.redAccent : Colors.cyanAccent,
                      fontSize: 10,
                      fontFamily: 'monospace',
                    ),
                  ),
                  const SizedBox(width: 16),
                  Text(
                    "RAM: ${(_currentRamMb / 1024).toStringAsFixed(1)} GB / ${(_totalRamMb / 1024).toStringAsFixed(1)} GB",
                    style: const TextStyle(color: Colors.white70, fontSize: 10, fontFamily: 'monospace'),
                  ),
                ],
              ),
            ),
            const SizedBox(height: 4),
            Expanded(
              child: ClipRect(
                child: CustomPaint(
                  size: const Size(double.infinity, double.infinity),
                  painter: _RollingGraphPainter(
                    dspData: _dspHistory.toList(),
                    cpuData: _cpuHistory.toList(),
                  ),
                ),
              ),
            ),
          ],
        ),
      ),
    );
  }
}

class _RollingGraphPainter extends CustomPainter {
  final List<double> dspData;
  final List<double> cpuData;

  _RollingGraphPainter({required this.dspData, required this.cpuData});

  @override
  void paint(Canvas canvas, Size size) {
    final width = size.width;
    final height = size.height;
    final stepX = width / (dspData.length - 1);

    // CPU Line (Background/Dim)
    final cpuPath = Path();
    final cpuPaint = Paint()
      ..color = Colors.white.withAlpha(50)
      ..strokeWidth = 1.0
      ..style = PaintingStyle.stroke;

    // DSP Line (Foreground/Bright)
    final dspPath = Path();
    final dspPaint = Paint()
      ..color = Colors.cyanAccent
      ..strokeWidth = 1.5
      ..style = PaintingStyle.stroke;

    for (int i = 0; i < dspData.length; i++) {
      final x = i * stepX;

      // Map 0-100% to Y coordinates (inverted so 100% is at the top)
      final dspY = height - ((dspData[i] / 100.0) * height);
      final cpuY = height - ((cpuData[i] / 100.0) * height);

      if (i == 0) {
        dspPath.moveTo(x, dspY);
        cpuPath.moveTo(x, cpuY);
      } else {
        dspPath.lineTo(x, dspY);
        cpuPath.lineTo(x, cpuY);
      }
    }

    canvas.drawPath(cpuPath, cpuPaint);
    canvas.drawPath(dspPath, dspPaint);
  }

  @override
  bool shouldRepaint(covariant _RollingGraphPainter oldDelegate) => true;
}
