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
    final audioSources = context
        .select<KarbeatState, Map<int, AudioWaveformUiForAudioProperties>>(
          (state) => state.audioSources,
        );

    final generators = context
        .select<KarbeatState, Map<int, UiGeneratorInstance>>(
          (value) => value.generators,
        );

    return Scaffold(
      backgroundColor: Colors.grey.shade900,
      floatingActionButton: FloatingActionButton(
        onPressed: () => _pickFile(context),
        backgroundColor: Colors.cyanAccent,
        child: const Icon(Icons.add),
      ),
      body: CustomScrollView(
        slivers: [
          // 1. GENERATORS SECTION
          const SliverToBoxAdapter(
            child: Padding(
              padding: EdgeInsets.fromLTRB(16, 16, 16, 8),
              child: Text("Instruments / Generators", style: TextStyle(color: Colors.white54, fontSize: 12, fontWeight: FontWeight.bold)),
            ),
          ),
          
          if (generators.isEmpty)
            const SliverToBoxAdapter(
              child: Padding(
                padding: EdgeInsets.all(16.0),
                child: Text("No Instruments.", style: TextStyle(color: Colors.grey, fontStyle: FontStyle.italic)),
              ),
            ),

          SliverList(
            delegate: SliverChildBuilderDelegate(
              (context, index) {
                final id = generators.keys.elementAt(index);
                final gen = generators.values.elementAt(index);
                return _SourceTile(
                  title: gen.name,
                  subtitle: "ID: $id | ${gen.internalType}",
                  icon: Icons.piano,
                  color: Colors.orangeAccent,
                  onTap: () {
                    // Navigate to Plugin Editor
                    // Navigator.of(context).push(
                    //   MaterialPageRoute(
                    //     builder: (_) => PluginEditorScreen(generatorId: id),
                    //   ),
                    // );
                  },
                  onPlace: () => context.read<KarbeatState>().startPlacement(id),
                  // onDelete: () => context.read<KarbeatState>().removeGenerator(id), // TODO implement
                );
              },
              childCount: generators.length,
            ),
          ),

          const SliverToBoxAdapter(child: Divider(color: Colors.grey)),

          // 2. AUDIO CLIPS SECTION
          const SliverToBoxAdapter(
            child: Padding(
              padding: EdgeInsets.fromLTRB(16, 8, 16, 8),
              child: Text("Audio Clips", style: TextStyle(color: Colors.white54, fontSize: 12, fontWeight: FontWeight.bold)),
            ),
          ),

          if (audioSources.isEmpty)
            const SliverToBoxAdapter(
              child: Padding(
                padding: EdgeInsets.all(16.0),
                child: Text("No Audio Files.", style: TextStyle(color: Colors.grey, fontStyle: FontStyle.italic)),
              ),
            ),

          SliverList(
            delegate: SliverChildBuilderDelegate(
              (context, index) {
                final id = audioSources.keys.elementAt(index);
                final source = audioSources.values.elementAt(index);
                return _SourceTile(
                  title: source.name,
                  subtitle: "ID: $id | ${source.sampleRate} Hz",
                  icon: Icons.audio_file,
                  color: Colors.cyanAccent,
                  onTap: () {
                    Navigator.of(context).push(
                      MaterialPageRoute(
                        builder: (_) => AudioPropertiesScreen(
                          sourceId: id,
                          sourceName: source.name,
                        ),
                      ),
                    );
                  },
                  onPlace: () => context.read<KarbeatState>().startPlacement(id),
                );
              },
              childCount: audioSources.length,
            ),
          ),
          
          // Extra padding at bottom for FAB
          const SliverToBoxAdapter(child: SizedBox(height: 80)),
        ],
      ),    
    );
  }
}

class _SourceTile extends StatelessWidget {
  final String title;
  final String subtitle;
  final IconData icon;
  final Color color;
  final VoidCallback onTap;
  final VoidCallback onPlace;

  const _SourceTile({
    required this.title,
    required this.subtitle,
    required this.icon,
    required this.color,
    required this.onTap,
    required this.onPlace,
  });

  @override
  Widget build(BuildContext context) {
    return ListTile(
      leading: Icon(icon, color: color),
      title: Text(title, style: const TextStyle(color: Colors.white)),
      subtitle: Text(subtitle, style: const TextStyle(color: Colors.grey)),
      trailing: PopupMenuButton<String>(
        icon: const Icon(Icons.more_vert, color: Colors.white),
        onSelected: (value) {
          if (value == 'place') onPlace();
        },
        itemBuilder: (context) => [
          const PopupMenuItem(
            value: 'place',
            child: Row(
              children: [
                Icon(Icons.input, color: Colors.black54),
                SizedBox(width: 8),
                Text("Put in timeline"),
              ],
            ),
          ),
          const PopupMenuItem(
            value: 'delete',
            child: Row(
              children: [
                Icon(Icons.delete, color: Colors.red),
                SizedBox(width: 8),
                Text("Delete"),
              ],
            ),
          ),
        ],
      ),
      onTap: onTap,
    );
  }
}