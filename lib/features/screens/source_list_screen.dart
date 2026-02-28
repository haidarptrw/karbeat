import 'package:file_picker/file_picker.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:karbeat/features/plugins/dynamic_plugin_screen.dart';
import 'package:karbeat/features/screens/audio_properties_screen.dart';
import 'package:karbeat/src/rust/api/track.dart';
import 'package:karbeat/state/app_state.dart';

class SourceListScreen extends ConsumerWidget {
  const SourceListScreen({super.key});

  Future<void> _pickFile(WidgetRef ref) async {
    FilePickerResult? result = await FilePicker.platform.pickFiles(
      type: FileType.audio,
    );

    if (result != null && result.files.single.path != null) {
      String path = result.files.single.path!;
      await ref.read(karbeatStateProvider).addAudioFile(path);
    }
  }

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    // Access the source map from state
    final audioSources = ref.watch(
      karbeatStateProvider.select((s) => s.audioSources),
    );

    final generators = ref.watch(
      karbeatStateProvider.select((s) => s.generators),
    );

    final patterns = ref.watch(karbeatStateProvider.select((s) => s.patterns));

    return Scaffold(
      backgroundColor: Colors.grey.shade900,
      floatingActionButton: FloatingActionButton(
        onPressed: () => _pickFile(ref),
        backgroundColor: Colors.cyanAccent,
        child: const Icon(Icons.add),
      ),
      body: CustomScrollView(
        slivers: [
          // 1. GENERATORS SECTION
          const SliverToBoxAdapter(
            child: Padding(
              padding: EdgeInsets.fromLTRB(16, 16, 16, 8),
              child: Text(
                "Instruments / Generators",
                style: TextStyle(
                  color: Colors.white54,
                  fontSize: 12,
                  fontWeight: FontWeight.bold,
                ),
              ),
            ),
          ),

          if (generators.isEmpty)
            const SliverToBoxAdapter(
              child: Padding(
                padding: EdgeInsets.all(16.0),
                child: Text(
                  "No Instruments.",
                  style: TextStyle(
                    color: Colors.grey,
                    fontStyle: FontStyle.italic,
                  ),
                ),
              ),
            ),

          SliverList(
            delegate: SliverChildBuilderDelegate((context, index) {
              final id = generators.keys.elementAt(index);
              final gen = generators.values.elementAt(index);
              return _SourceTile(
                title: gen.name,
                subtitle: "ID: $id",
                icon: Icons.piano,
                color: Colors.orangeAccent,
                onTap: () {
                  // Navigate to dynamic plugin editor for all generator types
                  Navigator.of(context).push(
                    MaterialPageRoute(
                      builder: (_) => DynamicPluginScreen(
                        generatorId: id,
                        generatorName: gen.name,
                      ),
                    ),
                  );
                },
                onPlace: null,
                // onDelete: () => ref.read(karbeatStateProvider).removeGenerator(id), // TODO implement
              );
            }, childCount: generators.length),
          ),

          const SliverToBoxAdapter(child: Divider(color: Colors.grey)),

          // 2. AUDIO CLIPS SECTION
          const SliverToBoxAdapter(
            child: Padding(
              padding: EdgeInsets.fromLTRB(16, 8, 16, 8),
              child: Text(
                "Audio Clips",
                style: TextStyle(
                  color: Colors.white54,
                  fontSize: 12,
                  fontWeight: FontWeight.bold,
                ),
              ),
            ),
          ),

          if (audioSources.isEmpty)
            const SliverToBoxAdapter(
              child: Padding(
                padding: EdgeInsets.all(16.0),
                child: Text(
                  "No Audio Files.",
                  style: TextStyle(
                    color: Colors.grey,
                    fontStyle: FontStyle.italic,
                  ),
                ),
              ),
            ),

          SliverList(
            delegate: SliverChildBuilderDelegate((context, index) {
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
                onPlace: () => ref
                    .read(karbeatStateProvider)
                    .startPlacement(id, type: UiSourceType.audio),
              );
            }, childCount: audioSources.length),
          ),

          const SliverToBoxAdapter(child: Divider(color: Colors.grey)),

          // Patterns list
          const SliverToBoxAdapter(
            child: Padding(
              padding: EdgeInsets.fromLTRB(16, 8, 16, 8),
              child: Text(
                "Patterns",
                style: TextStyle(
                  color: Colors.white54,
                  fontSize: 12,
                  fontWeight: FontWeight.bold,
                ),
              ),
            ),
          ),

          if (patterns.isEmpty)
            const SliverToBoxAdapter(
              child: Padding(
                padding: EdgeInsets.all(16.0),
                child: Text(
                  "No Patterns.",
                  style: TextStyle(
                    color: Colors.grey,
                    fontStyle: FontStyle.italic,
                  ),
                ),
              ),
            ),

          SliverList(
            delegate: SliverChildBuilderDelegate((context, index) {
              final id = patterns.keys.elementAt(index);
              final pattern = patterns.values.elementAt(index);
              return _SourceTile(
                title: pattern.name,
                subtitle: "ID: $id | ${pattern.name}",
                icon: Icons.music_note,
                color: Colors.purpleAccent,
                onTap: () {
                  ref.read(karbeatStateProvider).openPattern(id);
                },
                onPlace: () => ref
                    .read(karbeatStateProvider)
                    .startPlacement(id, type: UiSourceType.midi),
              );
            }, childCount: patterns.length),
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
  final VoidCallback? onPlace;

  const _SourceTile({
    required this.title,
    required this.subtitle,
    required this.icon,
    required this.color,
    required this.onTap,
    this.onPlace,
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
          if (value == 'place') onPlace?.call();
        },
        itemBuilder: (context) => [
          if (onPlace != null)
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
