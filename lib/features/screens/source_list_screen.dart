import 'package:file_picker/file_picker.dart';
import 'package:flutter/material.dart';
import 'package:karbeat/features/screens/audio_properties_screen.dart';
import 'package:karbeat/src/rust/api/project.dart';
import 'package:karbeat/state/app_state.dart';
import 'package:provider/provider.dart';

class SourceListScreen extends StatelessWidget {
  const SourceListScreen({super.key});

  Future<void> _pickFile(BuildContext context) async {
    FilePickerResult? result = await FilePicker.platform.pickFiles(
      type: FileType.audio,
    );

    if (result != null && result.files.single.path != null) {
      String path = result.files.single.path!;
      // Call state logic to load file via Rust
      if (context.mounted) {
        await context.read<KarbeatState>().addAudioFile(path);
      }
    }
  }

  @override
  Widget build(BuildContext context) {
    // Access the source map from state
    final sources = context
        .select<KarbeatState, Map<int, AudioWaveformUiForAudioProperties>>(
          (state) => state.audioSources,
        );

    return Scaffold(
      backgroundColor: Colors.grey.shade900,
      floatingActionButton: FloatingActionButton(
        onPressed: () => _pickFile(context),
        backgroundColor: Colors.cyanAccent,
        child: const Icon(Icons.add),
      ),
      body: sources.isEmpty
          ? const Center(
              child: Text(
                "No Audio Sources Loaded.\nClick + to add a WAV/MP3 file.",
                textAlign: TextAlign.center,
                style: TextStyle(color: Colors.grey),
              ),
            )
          : ListView.separated(
              padding: const EdgeInsets.all(16),
              itemCount: sources.length,
              separatorBuilder: (_, _) => const Divider(color: Colors.grey),
              itemBuilder: (context, index) {
                final id = sources.keys.elementAt(index);
                final sourceUi = sources.values.elementAt(index);

                return ListTile(
                  leading: const Icon(
                    Icons.audio_file,
                    color: Colors.cyanAccent,
                  ),
                  title: Text(
                    sourceUi.name,
                    style: const TextStyle(color: Colors.white),
                  ),
                  subtitle: Text(
                    "ID: $id",
                    style: TextStyle(color: Colors.grey.shade600),
                  ),
                  trailing: Row(
                    mainAxisSize: MainAxisSize.min,
                    children: [
                      IconButton(
                        icon: Icon(
                          sourceUi.muted ? Icons.volume_off : Icons.volume_up,
                          color: sourceUi.muted ? Colors.red : Colors.green,
                        ),
                        onPressed: () {
                          // TODO: Source Action (Drag and Drop to track timeline)
                        },
                      ),
                      const SizedBox(width: 8),
                      PopupMenuButton<String>(
                        icon: const Icon(Icons.more_vert, color: Colors.white),
                        onSelected: (value) {
                          if (value == 'place') {
                            // Trigger Placement Mode
                            context.read<KarbeatState>().startPlacement(id);
                          }
                        },
                        itemBuilder: (context) => [
                          const PopupMenuItem(
                            value: 'place',
                            child: Row(
                              children: [
                                Icon(Icons.input, color: Colors.black54),
                                SizedBox(width: 8),
                                Text("Put clip in timeline"),
                              ],
                            ),
                          ),
                          const PopupMenuItem(
                            value: 'delete',
                            child: Row(
                              children: [
                                Icon(Icons.delete, color: Colors.red),
                                SizedBox(width: 8),
                                Text("Delete Source"),
                              ],
                            ),
                          ),
                        ],
                      ),
                    ],
                  ),
                  onTap: () {
                    Navigator.of(context).push(
                      MaterialPageRoute(
                        builder: (context) => AudioPropertiesScreen(
                          sourceId: id, // Pass the ID from the map
                          sourceName: sourceUi.name,
                        ),
                      ),
                    );
                  },
                );
              },
            ),
    );
  }
}
