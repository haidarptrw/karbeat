import 'package:flutter/material.dart';
import 'package:karbeat/features/audio_plugins/generators/abstract_generator_screen.dart';

class MyRetroSynth extends AbstractGeneratorScreen {
  MyRetroSynth({required super.generatorIdx});

  @override
  MyRetroSynthState createState() => MyRetroSynthState();
}

class MyRetroSynthState extends AbstractGeneratorScreenState<MyRetroSynth> {
  /// build oscillators settings
  /// Available parameters
  /// - Waveform shape
  /// - mix
  /// - phase
  /// - 
  Widget _buildOscillatorSection({
    required int oscIndex,
    required int baseParamId,
  }) {

    return Container();
  }

  // add virtual piano keys

  @override
  Widget buildGeneratorBody(BuildContext context) {
    // TODO: implement buildGeneratorBody
    throw UnimplementedError();
  }
}

class WaveShapeDrawer extends CustomPainter {
  @override
  void paint(Canvas canvas, Size size) {
    // TODO: implement paint
  }

  @override
  bool shouldRepaint(covariant CustomPainter oldDelegate) {
    // TODO: implement shouldRepaint
    throw UnimplementedError();
  }
}
