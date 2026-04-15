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
}
