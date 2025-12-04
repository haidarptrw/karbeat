import 'package:flutter/material.dart';
import 'package:karbeat/features/playlist/track_slot.dart';
import 'package:karbeat/src/rust/api/project.dart';
import 'package:karbeat/src/rust/core/project.dart';
import 'package:karbeat/state/app_state.dart';
import 'package:linked_scroll_controller/linked_scroll_controller.dart';
import 'package:provider/provider.dart';

class TrackListScreen extends StatelessWidget {
  const TrackListScreen({super.key});

  @override
  Widget build(BuildContext context) {
    return LayoutBuilder(
      builder: (context, constraints) {
        final parentHeight = constraints.maxHeight;
        if (parentHeight.isInfinite) return const SizedBox();

        final calculatedHeight = parentHeight * 0.20;
        final double itemHeight = calculatedHeight.clamp(80.0, 150.0);
        const double headerWidth = 220.0;

        return Consumer<KarbeatState>(
          builder: (context, state, child) {
            final tracks = state.tracks.values.toList()
              ..sort((a, b) => a.id.compareTo(b.id));

            return _SplitTrackView(
              tracks: tracks,
              itemHeight: itemHeight,
              headerWidth: headerWidth,
            );
          },
        );
      },
    );
  }
}

class _SplitTrackView extends StatefulWidget {
  final List<UiTrack> tracks;
  final double itemHeight;
  final double headerWidth;

  const _SplitTrackView({
    required this.tracks,
    required this.itemHeight,
    required this.headerWidth,
  });

  @override
  State<_SplitTrackView> createState() => _SplitTrackViewState();
}

class _SplitTrackViewState extends State<_SplitTrackView> {
  late LinkedScrollControllerGroup _verticalControllers;
  late ScrollController _headerController;
  late ScrollController _timelineController;

  @override
  void initState() {
    super.initState();
    // Initialize the Linked Group
    _verticalControllers = LinkedScrollControllerGroup();
    _headerController = _verticalControllers.addAndGet();
    _timelineController = _verticalControllers.addAndGet();
  }

  @override
  void dispose() {
    _headerController.dispose();
    _timelineController.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    // Calculate total height to ensure both lists have exactly same extent
    // +1 for the Add Button row
    final int itemCount = widget.tracks.length + 1;

    return Row(
      children: [
        // --- LEFT: TRACK HEADERS ---
        SizedBox(
          width: widget.headerWidth,
          child: Column(
            children: [
              // Optional: Fixed Header Row (e.g. "Name", "Mute")
              Container(
                height: 30,
                color: Colors.grey.shade800,
                alignment: Alignment.centerLeft,
                padding: const EdgeInsets.only(left: 10),
                child: const Text("Tracks", style: TextStyle(color: Colors.white70, fontSize: 12)),
              ),
              Expanded(
                child: ListView.builder(
                  controller: _headerController, // Controller 1
                  padding: EdgeInsets.zero,
                  itemCount: itemCount,
                  itemBuilder: (context, index) {
                    if (index == widget.tracks.length) {
                      return _buildAddButton();
                    }
                    return _buildTrackHeader(widget.tracks[index]);
                  },
                ),
              ),
            ],
          ),
        ),

        // --- DIVIDER ---
        Container(width: 1, color: Colors.black),

        // --- RIGHT: TIMELINE ---
        Expanded(
          child: Column(
            children: [
              // Optional: Time Ruler Header (Horizontal Scrollable)
              // We would need another sync controller for the ruler + body horizontal scroll.
              Container(
                height: 30,
                color: Colors.grey.shade800,
                width: double.infinity,
                // Placeholder for Ruler
              ),
              Expanded(
                child: SingleChildScrollView(
                  scrollDirection: Axis.horizontal, // Horizontal Scroll
                  // Physics to match desktop feel
                  physics: const ClampingScrollPhysics(), 
                  child: SizedBox(
                    width: 50000, // TODO: Bind to project duration
                    child: ListView.builder(
                      controller: _timelineController, // Controller 2 (Synced Vertically)
                      padding: EdgeInsets.zero,
                      itemCount: itemCount,
                      itemBuilder: (context, index) {
                        if (index == widget.tracks.length) {
                          // Empty space matching Add Button height
                          return SizedBox(height: 60); 
                        }
                        return KarbeatTrackSlot(
                          trackId: widget.tracks[index].id,
                          height: widget.itemHeight,
                        );
                      },
                    ),
                  ),
                ),
              ),
            ],
          ),
        ),
      ],
    );
  }

  Widget _buildAddButton() {
    return SizedBox(
      height: 60,
      child: Center(
        child: TextButton.icon(
          onPressed: () => _showAddTrackDialog(context),
          icon: const Icon(Icons.add, color: Colors.white54),
          label: const Text("Add New Track", style: TextStyle(color: Colors.white54)),
        ),
      ),
    );
  }

  Widget _buildTrackHeader(UiTrack track) {
    return SizedBox(
      height: widget.itemHeight,
      child: Container(
        margin: const EdgeInsets.only(bottom: 2),
        padding: const EdgeInsets.symmetric(horizontal: 10),
        decoration: BoxDecoration(
          color: Colors.grey.shade300,
          border: Border(
            bottom: BorderSide(color: Colors.grey.shade400, width: 1),
            right: BorderSide(color: Colors.grey.shade400, width: 1),
          ),
        ),
        child: Row(
          children: [
            Icon(_getTrackIcon(track.trackType), color: Colors.grey.shade700),
            const SizedBox(width: 10),
            Expanded(
              child: Column(
                mainAxisAlignment: MainAxisAlignment.center,
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text(
                    track.name,
                    style: TextStyle(
                      color: Colors.grey.shade800,
                      fontWeight: FontWeight.bold,
                      fontSize: 14,
                    ),
                    overflow: TextOverflow.ellipsis,
                  ),
                  Text(
                    "ID: ${track.id} | ${track.trackType.name.toUpperCase()}",
                    style: TextStyle(color: Colors.grey.shade600, fontSize: 10),
                  ),
                ],
              ),
            ),
            Column(
              mainAxisAlignment: MainAxisAlignment.center,
              children: [
                InkWell(onTap: () {}, child: const Icon(Icons.mic_off, size: 16, color: Colors.grey)),
                const SizedBox(height: 4),
                InkWell(onTap: () {}, child: const Icon(Icons.volume_up, size: 16, color: Colors.grey)),
              ],
            )
          ],
        ),
      ),
    );
  }

  IconData _getTrackIcon(TrackType type) {
    switch (type) {
      case TrackType.audio: return Icons.graphic_eq;
      case TrackType.midi: return Icons.piano;
      case TrackType.automation: return Icons.show_chart;
    }
  }

  void _showAddTrackDialog(BuildContext context) {
    showDialog(
      context: context,
      builder: (ctx) => SimpleDialog(
        title: const Text("Select Track Type"),
        children: [
          SimpleDialogOption(
            onPressed: () {
              context.read<KarbeatState>().addTrack(TrackType.audio);
              Navigator.pop(ctx);
            },
            child: const Text("Audio Track"),
          ),
          SimpleDialogOption(
            onPressed: () {
              context.read<KarbeatState>().addTrack(TrackType.midi);
              Navigator.pop(ctx);
            },
            child: const Text("MIDI / Instrument Track"),
          ),
        ],
      ),
    );
  }
}