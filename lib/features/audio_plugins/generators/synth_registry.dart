import 'package:karbeat/features/audio_plugins/generators/abstract_generator_screen.dart';
import 'package:karbeat/features/audio_plugins/generators/karbeatzer_screen.dart';
import 'package:karbeat/features/audio_plugins/generators/my_retro_synth.dart';
import 'package:karbeat/src/rust/api/utils.dart';

typedef GeneratorScreenBuilder =
    AbstractGeneratorScreen Function(int generatorId);

class SynthRegistry {

  SynthRegistry._();

  static final karbeatzerId = hashStrFnv1A(s: "synth_karbeatzer_v2");
  static final myRetroId = hashStrFnv1A(s: "synth_my_retro");
  static final Map<int, GeneratorScreenBuilder> _synths = {
    karbeatzerId: (id) => KarbeatzerScreen(generatorId: id),
    1: (id) => MyRetroSynth(generatorId: id),
  };

  /// Retrieves the correct screen widget for a specific plugin.
  /// 
  /// [registryId] is the stable FNV-1a hash identifying the plugin type.
  /// [instanceId] is the unique ID of the specific generator allocated in the project.
  static AbstractGeneratorScreen getScreen({
    required int registryId,
    required int instanceId,
  }) {
    final builder = _synths[registryId];
    
    if (builder == null) {
      throw Exception(
        'UI Screen not found for Registry ID $registryId. '
        'Did you forget to add it to SynthRegistry?'
      );
    }
    
    return builder(instanceId);
  }
}
