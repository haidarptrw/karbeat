import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:karbeat/features/components/waveform_painter.dart';
import 'package:karbeat/src/rust/api/audio.dart';
import 'package:karbeat/src/rust/api/project.dart';

class AudioPropertiesScreen extends ConsumerWidget {
  final int sourceId;
  final String sourceName;

  AudioPropertiesScreen({
    super.key,
    required this.sourceId,
    required this.sourceName,
  });

  final audioPropertiesProvider = FutureProvider.autoDispose
      .family<AudioWaveformUiForAudioProperties, int>((ref, sourceId) async {
        final result = await getAudioProperties(id: sourceId);

        if (result == null) {
          throw Exception("Failed to load audio properties");
        }

        return result;
      });

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final propsAsync = ref.watch(audioPropertiesProvider(sourceId));

    return Scaffold(
      backgroundColor: Colors.grey.shade900,
      appBar: AppBar(
        title: Text(sourceName),
        backgroundColor: const Color.fromARGB(255, 9, 7, 7),
        elevation: 0,
      ),
      body: propsAsync.when(
        loading: () => const Center(child: CircularProgressIndicator()),

        error: (err, _) => Center(
          child: Text("Error: $err", style: const TextStyle(color: Colors.red)),
        ),

        data: (props) {
          return Column(
            children: [
              // HEADER
              _buildInfoSection(props),

              const Divider(color: Colors.grey),

              // WAVEFORM
              Expanded(
                child: Padding(
                  padding: const EdgeInsets.all(16.0),
                  child: Container(
                    width: double.infinity,
                    decoration: BoxDecoration(
                      color: Colors.black,
                      border: Border.all(color: Colors.grey.shade700),
                      borderRadius: BorderRadius.circular(8),
                    ),
                    child: ClipRRect(
                      borderRadius: BorderRadius.circular(8),
                      child: CustomPaint(
                        painter: StereoWaveformPainter(
                          samples: props.previewBuffer,
                          color: Colors.cyanAccent,
                        ),
                      ),
                    ),
                  ),
                ),
              ),

              // CONTROLS
              Container(
                padding: const EdgeInsets.all(24),
                color: Colors.grey.shade800,
                child: Row(
                  mainAxisAlignment: MainAxisAlignment.center,
                  children: [
                    FloatingActionButton.extended(
                      heroTag: 'play_source_fab',
                      onPressed: () {
                        playSourcePreview(id: sourceId);
                      },
                      icon: const Icon(Icons.play_arrow),
                      label: const Text("Preview"),
                      backgroundColor: Colors.cyanAccent,
                      foregroundColor: Colors.black,
                    ),
                    const SizedBox(width: 10),
                    FloatingActionButton.extended(
                      heroTag: 'stop_source_fab',
                      onPressed: () {
                        stopAllPreviews();
                      },
                      label: const Text("Stop"),
                      icon: const Icon(Icons.stop),
                      backgroundColor: Colors.redAccent,
                      foregroundColor: Colors.black,
                    ),
                  ],
                ),
              ),
            ],
          );
        },
      ),
    );
  }

  Widget _buildInfoSection(AudioWaveformUiForAudioProperties props) {
    return Padding(
      padding: const EdgeInsets.all(16.0),
      child: Column(
        children: [
          _row("Format", "${props.sampleRate} Hz / ${props.channels} Ch"),
          _row("Duration", "${props.duration.toStringAsFixed(2)} sec"),
          _row("Path", props.filePath, isSmall: true),
        ],
      ),
    );
  }

  Widget _row(String label, String value, {bool isSmall = false}) {
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 4.0),
      child: Row(
        mainAxisAlignment: MainAxisAlignment.spaceBetween,
        children: [
          Text(label, style: const TextStyle(color: Colors.grey)),
          Flexible(
            child: Text(
              value,
              style: TextStyle(
                color: Colors.white,
                fontSize: isSmall ? 10 : 14,
                overflow: TextOverflow.ellipsis,
              ),
            ),
          ),
        ],
      ),
    );
  }
}
