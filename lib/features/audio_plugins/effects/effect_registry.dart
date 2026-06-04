import 'package:karbeat/features/audio_plugins/effects/abstract_effect_screen.dart';
import 'package:karbeat/features/audio_plugins/effects/karbeat_parametric_eq.dart';
import 'package:karbeat/src/rust/api/plugin.dart';
import 'package:karbeat/src/rust/api/utils.dart';

typedef EffectScreenBuilder =
    AbstractEffectScreen Function(int effectId, UiEffectTarget target);

class EffectRegistry {
  EffectRegistry._();
  static final Map<int, EffectScreenBuilder> _effects = {
    hashStrFnv1A(s: "effect_param_eq"): (id, target) =>
        KarbeatParametricEq(effectId: id, target: target),
  };

  static AbstractEffectScreen getScreen({
    required int registryId,
    required int instanceId,
    required UiEffectTarget target
  }) {
    final builder = _effects[registryId];
    
    if (builder == null) {
      throw Exception(
        'UI Screen not found for Registry ID $registryId. '
        'Did you forget to add it to EffectRegistry?'
      );
    }
    
    return builder(instanceId, target);
  }
}
