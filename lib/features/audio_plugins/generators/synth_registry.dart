import 'package:karbeat/features/audio_plugins/generators/abstract_generator_screen.dart';
import 'package:karbeat/features/audio_plugins/generators/karbeatzer_screen.dart';
import 'package:karbeat/features/audio_plugins/generators/my_retro_synth.dart';

typedef GeneratorScreenBuilder =
    AbstractGeneratorScreen Function(int generatorId);

class SynthRegistry {
  static final Map<int, GeneratorScreenBuilder> _synths = {
    0: (id) => KarbeatzerScreen(generatorId: id),
    1: (id) => MyRetroSynth(generatorId: id),
  };

  static GeneratorScreenBuilder getSynthBuilder(int generatorId) {
    if (!_synths.containsKey(generatorId)) {
      throw Exception('Generator screen for ID $generatorId not registered');
    }
    return _synths[generatorId]!;
  }
}
