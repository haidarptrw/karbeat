import 'package:flutter/material.dart';
import 'package:karbeat/features/components/waveform_painter.dart';
import 'package:karbeat/src/rust/api/audio.dart';
import 'package:karbeat/src/rust/api/project.dart';

class AudioPropertiesScreen extends StatefulWidget {
  final int sourceId;
  final String sourceName;

  const AudioPropertiesScreen({
    super.key,
    required this.sourceId,
    required this.sourceName,
  });

  @override
  State<AudioPropertiesScreen> createState() => _AudioPropertiesScreenState();
}

class _AudioPropertiesScreenState extends State<AudioPropertiesScreen> {
  Future<AudioWaveformUiForAudioProperties?>? _propertiesFuture;

  @override
  void initState() {
    super.initState();
    _loadProperties();
  }

  void _loadProperties() {
    // Call Rust API to get heavy data (waveform buffer)
    setState(() {
      _propertiesFuture = getAudioProperties(id: widget.sourceId);
    });
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      backgroundColor: Colors.grey.shade900,
      appBar: AppBar(
        title: Text(widget.sourceName),
        backgroundColor: Colors.grey.shade800,
        elevation: 0,
      ),
      body: FutureBuilder<AudioWaveformUiForAudioProperties?>(
        future: _propertiesFuture,
        builder: (context, snapshot) {
          if (snapshot.connectionState == ConnectionState.waiting) {
            return const Center(child: CircularProgressIndicator());
          }

          if (snapshot.hasError || !snapshot.hasData || snapshot.data == null) {
            return const Center(
              child: Text(
                "Failed to load audio properties",
                style: TextStyle(color: Colors.white),
              ),
            );
          }

          final props = snapshot.data!;

          return Column(
            children: [
              // ========== HEADER INFO ==================
              _buildInfoSection(props),

              const Divider(color: Colors.grey),

              // ========== WAVEFORM DISPLAY ===============
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
                          samples:
                              props.previewBuffer, // The Float32List from Rust
                          color: Colors.cyanAccent,
                        ),
                      ),
                    ),
                  ),
                ),
              ),

              // ============ CONTROLS ===============
              Container(
                padding: const EdgeInsets.all(24),
                color: Colors.grey.shade800,
                child: Row(
                  mainAxisAlignment: MainAxisAlignment.center,
                  children: [
                    FloatingActionButton.extended(
                      heroTag: 'play_source_fab',
                      onPressed: () {
                        playSourcePreview(id: widget.sourceId);
                      },
                      icon: const Icon(Icons.play_arrow),
                      label: const Text("Preview"),
                      backgroundColor: Colors.cyanAccent,
                      foregroundColor: Colors.black,
                    ),
                    SizedBox(
                      width: 10,
                    ),
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
