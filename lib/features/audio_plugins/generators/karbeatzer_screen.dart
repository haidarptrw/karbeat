import 'package:karbeat/features/audio_plugins/generators/abstract_generator_screen.dart';

class KarbeatzerScreen extends AbstractGeneratorScreen {
  const KarbeatzerScreen({super.key, required super.generatorId});

  @override
  KarbeatzerScreenState createState() {
    return KarbeatzerScreenState();
  }
}

class KarbeatzerScreenState extends AbstractGeneratorScreenState<KarbeatzerScreen> {
  @override
  String get generatorName => 'Karbeatzer V2';

}