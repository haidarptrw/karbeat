import 'package:flutter/material.dart';
import 'package:karbeat/features/audio_plugins/generators/abstract_generator_screen.dart';

class MyRetroSynth extends AbstractGeneratorScreen {
  MyRetroSynth({required super.generatorId});

  @override
  MyRetroSynthState createState() => MyRetroSynthState();
}

class MyRetroSynthState extends AbstractGeneratorScreenState<MyRetroSynth> {
  @override
  String get generatorName => 'MyRetro Synth';

  /// Helper to safely fetch a parameter value from the inherited `parameters` list.
  /// Provides a fallback if the parameter hasn't loaded yet.
  double _getParamValue(int paramId, double fallback) {
    try {
      return parameters.firstWhere((p) => p.id == paramId).value;
    } catch (e) {
      return fallback;
    }
  }

  /// build oscillators settings
  /// Available parameters
  /// - Waveform shape
  /// - mix
  /// - phase offset
  /// - detune
  /// - pulse width
  Widget _buildOscillatorSection({
    required int oscIndex,
    required int baseParamId,
  }) {
    // 1. Calculate specific parameter IDs for this oscillator
    final waveformId = baseParamId + 0;
    final detuneId = baseParamId + 1;
    final mixId = baseParamId + 2;
    final pwId = baseParamId + 3;
    final phaseId = baseParamId + 4;

    // 2. Fetch current values from the audio thread state
    final waveformVal = _getParamValue(waveformId, 2.0).toInt();
    final detuneVal = _getParamValue(detuneId, oscIndex == 1 ? 0.0 : -12.0);
    final mixVal = _getParamValue(mixId, oscIndex == 1 ? 1.0 : 0.8);
    final pwVal = _getParamValue(pwId, 0.5);
    final phaseVal = _getParamValue(phaseId, 0.0);

    return Container();
  }

  // add virtual piano keys

  @override
  Widget buildGeneratorBody(BuildContext context) {
    // TODO: implement buildGeneratorBody
    throw UnimplementedError();
  }
}

class _WaveShapeDrawer extends CustomPainter {
  final int waveformType;
  _WaveShapeDrawer(this.waveformType);

  @override
  void paint(Canvas canvas, Size size) {
    // Just drawing the square wave for visual retro flavor
    if (waveformType == 2) {
      _drawSquareWaveform(canvas, size);
    }
  }

  @override
  bool shouldRepaint(covariant _WaveShapeDrawer oldDelegate) {
    return oldDelegate.waveformType != waveformType;
  }

  void _drawSquareWaveform(Canvas canvas, Size size) {
    final paint = Paint()
      ..color = Colors.greenAccent
      ..strokeWidth = 2.0
      ..style = PaintingStyle.stroke
      ..strokeJoin = StrokeJoin.miter;

    final path = Path();
    path.moveTo(0, size.height / 2);
    path.lineTo(0, 0);
    path.lineTo(size.width / 2, 0);
    path.lineTo(size.width / 2, size.height);
    path.lineTo(size.width, size.height);
    path.lineTo(size.width, size.height / 2);

    canvas.drawPath(path, paint);
  }
}
