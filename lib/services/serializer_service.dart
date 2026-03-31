import 'package:karbeat/src/rust/api/serialization.dart' as serialization_api;
import 'package:karbeat/src/rust/api/project.dart';

class SerializerService {
  Future<void> saveProject({required String pathName}) async {
    await serialization_api.saveProject(pathName: pathName);
  }

  Future<UiApplicationState> loadProject({required String pathName}) async {
    final uiState = await serialization_api.loadProject(pathName: pathName);
    return uiState;
  }
}