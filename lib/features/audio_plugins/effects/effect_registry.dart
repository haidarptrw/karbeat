import 'package:karbeat/features/audio_plugins/effects/abstract_effect_screen.dart';
import 'package:karbeat/features/audio_plugins/effects/karbeat_parametric_eq.dart';
import 'package:karbeat/src/rust/api/plugin.dart';

typedef EffectScreenBuilder =
    AbstractEffectScreen Function(int effectId, UiEffectTarget target);

class EffectRegistry {
  static final Map<int, EffectScreenBuilder> _effects = {
    0: (id, target) => KarbeatParametricEq(effectId: id, target: target),
  };

  static EffectScreenBuilder getEffectBuilder(int effectId) {
    if (!_effects.containsKey(effectId)) {
      throw Exception('Effect screen for ID $effectId not registered');
    }
    return _effects[effectId]!;
  }
}
