import 'package:flutter/gestures.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:karbeat/features/components/fine_grained_input.dart';
import 'package:karbeat/src/rust/api/audio.dart';
import 'package:karbeat/state/app_state.dart';
import 'package:karbeat/utils/formatter.dart';
import 'package:karbeat/utils/scroll_behavior.dart';

class ControlPanel extends StatelessWidget {
  final List<Widget> items;
  final Color backgroundColor;

  const ControlPanel({
    super.key,
    required this.items,
    this.backgroundColor = const Color(0xFF1E1E1E),
  });

  @override
  Widget build(BuildContext context) {
    return Container(
      width: double.infinity,
      decoration: BoxDecoration(
        color: backgroundColor,
        border: Border(bottom: BorderSide(color: Colors.grey.shade800)),
        boxShadow: const [
          BoxShadow(color: Colors.black26, blurRadius: 4, offset: Offset(0, 2)),
        ],
      ),
      padding: const EdgeInsets.symmetric(horizontal: 8.0),
      child: ScrollConfiguration(
        behavior: DragScrollBehavior(),
        child: SingleChildScrollView(
          scrollDirection: Axis.horizontal,
          physics: const AlwaysScrollableScrollPhysics(),
          child: Padding(
            padding: const EdgeInsets.symmetric(horizontal: 8.0),
            child: Row(
              crossAxisAlignment: CrossAxisAlignment.center,
              mainAxisSize: MainAxisSize.min, // Takes minimum space needed
              children: items,
            ),
          ),
        ),
      ),
    );
  }
}

class ControlPanelBuilder {
  final List<Widget> _items = [];

  void addItem(Widget item) {
    _items.add(item);
  }

  void addSpacer() {
    _items.add(const SizedBox(width: 16)); // Visual gap
  }

  void addDivider() {
    _items.add(
      Container(
        margin: const EdgeInsets.symmetric(horizontal: 8),
        width: 1,
        height: 30,
        color: Colors.grey.shade700,
      ),
    );
  }

  void addWidget(Widget widget) {
    _items.add(widget);
  }

  ControlPanel build() {
    return ControlPanel(items: _items);
  }
}

class ControlPanelToolbarItem extends StatelessWidget {
  final String name;
  final IconData icon;
  final Color color;
  final VoidCallback? onTap;
  final bool isActive;

  const ControlPanelToolbarItem({
    super.key,
    required this.name,
    required this.icon,
    required this.color,
    this.onTap,
    this.isActive = false,
  });

  @override
  Widget build(BuildContext context) {
    return Tooltip(
      message: name,
      child: Material(
        color: Colors.transparent,
        child: InkWell(
          onTap: onTap,
          borderRadius: BorderRadius.circular(4),
          child: Container(
            height: 50,
            padding: const EdgeInsets.symmetric(horizontal: 12),
            decoration: isActive
                ? BoxDecoration(
                    color: Colors.white.withAlpha(25),
                    borderRadius: BorderRadius.circular(4),
                    border: Border.all(color: color.withAlpha(25)),
                  )
                : null,
            child: Column(
              mainAxisAlignment: MainAxisAlignment.center,
              children: [
                Icon(
                  icon,
                  color: isActive ? color : color.withAlpha(165),
                  size: 20,
                ),
                const SizedBox(height: 2),
                Text(
                  name,
                  style: TextStyle(
                    color: isActive ? color : color.withAlpha(165),
                    fontSize: 10,
                  ),
                ),
              ],
            ),
          ),
        ),
      ),
    );
  }
}

/// A space-saving dropdown version of the ControlPanelToolbarItem
class ControlPanelDropdown<T> extends StatelessWidget {
  final String name;
  final IconData icon;
  final Color color;
  final ValueChanged<T> onSelected;
  final List<PopupMenuEntry<T>> items;

  const ControlPanelDropdown({
    super.key,
    required this.name,
    required this.icon,
    required this.color,
    required this.onSelected,
    required this.items,
  });

  @override
  Widget build(BuildContext context) {
    return PopupMenuButton<T>(
      tooltip: name,
      color: const Color(0xFF2A2A2A), // Dark popup background to match theme
      elevation: 8,
      position: PopupMenuPosition.under,
      onSelected: onSelected,
      itemBuilder: (context) => items,
      child: Container(
        height: 50,
        padding: const EdgeInsets.symmetric(horizontal: 8),
        child: Row(
          mainAxisSize: MainAxisSize.min,
          children: [
            Column(
              mainAxisAlignment: MainAxisAlignment.center,
              children: [
                Icon(icon, color: color, size: 20),
                const SizedBox(height: 2),
                Text(name, style: TextStyle(color: color, fontSize: 10)),
              ],
            ),
            const SizedBox(width: 4),
            Icon(Icons.arrow_drop_down, color: color.withAlpha(150), size: 16),
          ],
        ),
      ),
    );
  }
}

class DefaultControlPanel extends ConsumerWidget {
  const DefaultControlPanel({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final state = ref.watch(karbeatStateProvider);
    final builder = ControlPanelBuilder();

    // 1. Screen Navigation Dropdown
    builder.addItem(
      ControlPanelDropdown<WorkspaceView>(
        name: _getViewName(state.currentView),
        icon: _getViewIcon(state.currentView),
        color: Colors.cyanAccent,
        onSelected: (view) => ref.read(karbeatStateProvider).navigateTo(view),
        items: const [
          PopupMenuItem(
            value: WorkspaceView.trackList,
            child: ListTile(
              leading: Icon(Icons.view_list, color: Colors.cyanAccent),
              title: Text("Tracks", style: TextStyle(color: Colors.white)),
              contentPadding: EdgeInsets.zero,
            ),
          ),
          PopupMenuItem(
            value: WorkspaceView.pianoRoll,
            child: ListTile(
              leading: Icon(Icons.piano, color: Colors.cyanAccent),
              title: Text("Piano Roll", style: TextStyle(color: Colors.white)),
              contentPadding: EdgeInsets.zero,
            ),
          ),
          PopupMenuItem(
            value: WorkspaceView.mixer,
            child: ListTile(
              leading: Icon(Icons.tune, color: Colors.cyanAccent),
              title: Text("Mixer", style: TextStyle(color: Colors.white)),
              contentPadding: EdgeInsets.zero,
            ),
          ),
          PopupMenuItem(
            value: WorkspaceView.source,
            child: ListTile(
              leading: Icon(Icons.group_work, color: Colors.cyanAccent),
              title: Text("Source", style: TextStyle(color: Colors.white)),
              contentPadding: EdgeInsets.zero,
            ),
          ),
        ],
      ),
    );

    builder.addDivider();

    // 2. Transport Panel
    builder.addWidget(
      Row(
        mainAxisSize: MainAxisSize.min,
        children: [
          ControlPanelToolbarItem(
            name: state.isSongPlaying ? "Pause" : "Play",
            icon: state.isSongPlaying ? Icons.pause : Icons.play_arrow,
            color: Colors.greenAccent,
            isActive: state.isSongPlaying,
            onTap: () => ref.read(karbeatStateProvider).togglePlay(),
          ),
          ControlPanelToolbarItem(
            name: "Stop",
            icon: Icons.stop,
            color: Colors.redAccent,
            onTap: () => ref.read(karbeatStateProvider).stop(),
          ),
          ControlPanelToolbarItem(
            name: "Loop",
            icon: Icons.loop,
            color: Colors.orangeAccent,
            isActive: state.isLooping,
            onTap: () => ref.read(karbeatStateProvider).toggleLoop(),
          ),
        ],
      ),
    );

    builder.addWidget(
      Row(
        children: [
          ControlPanelToolbarItem(
            name: "Snap to Grid",
            icon: Icons.grid_on,
            color: Colors.blueAccent,
            isActive: state.snapToGrid,
            onTap: () => ref.read(karbeatStateProvider).toggleSnapToGrid(),
          ),
          const SizedBox(width: 8),
          ControlPanelToolbarItem(
            name: "MIDI KB",
            icon: Icons.piano,
            color: Colors.deepPurpleAccent,
            isActive: state.showFloatingMidiKeyboard,
            onTap: () => ref.read(karbeatStateProvider).toggleFloatingMidiKeyboard(),
          ),
        ],
      ),
    );
    builder.addDivider();

    // 3. Info Display
    builder.addWidget(_buildInfoDisplay(context, ref));

    builder.addDivider();

    // 4. Control Panel Tools Dropdown
    builder.addItem(
      ControlPanelDropdown<ToolSelection>(
        name: _getToolName(state.selectedTool),
        icon: _getToolIcon(state.selectedTool),
        color: state.selectedTool == ToolSelection.delete
            ? Colors.red
            : Colors.blueAccent,
        onSelected: (tool) => ref.read(karbeatStateProvider).selectTool(tool),
        items: const [
          PopupMenuItem(
            value: ToolSelection.pointer,
            child: ListTile(
              leading: Icon(Icons.near_me, color: Colors.blueAccent),
              title: Text("Pointer", style: TextStyle(color: Colors.white)),
              contentPadding: EdgeInsets.zero,
            ),
          ),
          PopupMenuItem(
            value: ToolSelection.cut,
            child: ListTile(
              leading: Icon(Icons.content_cut, color: Colors.blueAccent),
              title: Text("Cut", style: TextStyle(color: Colors.white)),
              contentPadding: EdgeInsets.zero,
            ),
          ),
          PopupMenuItem(
            value: ToolSelection.draw,
            child: ListTile(
              leading: Icon(Icons.edit, color: Colors.blueAccent),
              title: Text("Draw", style: TextStyle(color: Colors.white)),
              contentPadding: EdgeInsets.zero,
            ),
          ),
          PopupMenuItem(
            value: ToolSelection.move,
            child: ListTile(
              leading: Icon(Icons.open_with, color: Colors.blueAccent),
              title: Text("Move", style: TextStyle(color: Colors.white)),
              contentPadding: EdgeInsets.zero,
            ),
          ),
          PopupMenuItem(
            value: ToolSelection.delete,
            child: ListTile(
              leading: Icon(Icons.delete, color: Colors.red),
              title: Text("Delete", style: TextStyle(color: Colors.white)),
              contentPadding: EdgeInsets.zero,
            ),
          ),
          PopupMenuItem(
            value: ToolSelection.select,
            child: ListTile(
              leading: Icon(Icons.crop_free, color: Colors.blueAccent),
              title: Text(
                "Range Select",
                style: TextStyle(color: Colors.white),
              ),
              contentPadding: EdgeInsets.zero,
            ),
          ),
          PopupMenuItem(
            value: ToolSelection.resize,
            child: ListTile(
              leading: Icon(Icons.zoom_out_map, color: Colors.blueAccent),
              title: Text("Resize", style: TextStyle(color: Colors.white)),
              contentPadding: EdgeInsets.zero,
            ),
          ),
        ],
      ),
    );

    return builder.build();
  }

  // Helpers to dynamically display the currently selected View
  String _getViewName(WorkspaceView view) {
    switch (view) {
      case WorkspaceView.trackList:
        return "Tracks";
      case WorkspaceView.pianoRoll:
        return "Piano Roll";
      case WorkspaceView.mixer:
        return "Mixer";
      case WorkspaceView.source:
        return "Source";
    }
  }

  IconData _getViewIcon(WorkspaceView view) {
    switch (view) {
      case WorkspaceView.trackList:
        return Icons.view_list;
      case WorkspaceView.pianoRoll:
        return Icons.piano;
      case WorkspaceView.mixer:
        return Icons.tune;
      case WorkspaceView.source:
        return Icons.group_work;
    }
  }

  // Helpers to dynamically display the currently selected Tool
  String _getToolName(ToolSelection tool) {
    switch (tool) {
      case ToolSelection.pointer:
        return "Pointer";
      case ToolSelection.cut:
        return "Cut";
      case ToolSelection.draw:
        return "Draw";
      case ToolSelection.move:
        return "Move";
      case ToolSelection.delete:
        return "Delete";
      case ToolSelection.select:
        return "Select";
      case ToolSelection.resize:
        return "Resize";
      default:
        return "Pointer";
    }
  }

  IconData _getToolIcon(ToolSelection tool) {
    switch (tool) {
      case ToolSelection.pointer:
        return Icons.near_me;
      case ToolSelection.cut:
        return Icons.content_cut;
      case ToolSelection.draw:
        return Icons.edit;
      case ToolSelection.move:
        return Icons.open_with;
      case ToolSelection.delete:
        return Icons.delete;
      case ToolSelection.select:
        return Icons.crop_free;
      case ToolSelection.resize:
        return Icons.zoom_out_map;
      default:
        return Icons.near_me;
    }
  }

  Widget _buildInfoDisplay(BuildContext context, WidgetRef ref) {
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 4),
      decoration: BoxDecoration(
        color: Colors.black54,
        borderRadius: BorderRadius.circular(4),
        border: Border.all(color: Colors.grey.shade700),
      ),
      child: IntrinsicHeight(
        child: StreamBuilder<UiTransportFeedback>(
          stream: ref.read(karbeatStateProvider).positionStream,
          builder: (context, asyncSnapshot) {
            final pos = asyncSnapshot.data;
            final bar = pos?.bar ?? 0;
            final beat = pos?.beat ?? 0;
            final samples = pos?.samples ?? 0;
            final sampleRate = pos?.sampleRate ?? 44100;
            return Row(
              children: [
                _buildInfoText("BAR", bar.toString()),
                const SizedBox(width: 10),
                _buildInfoText("BEAT", beat.toString()),
                const VerticalDivider(color: Colors.grey, width: 20),
                _buildInfoText(
                  "TIME",
                  formatTimeFromSamples(samples, sampleRate),
                ),
                const VerticalDivider(color: Colors.grey, width: 20),
                const BpmControl(),
                const VerticalDivider(color: Colors.grey, width: 20),
                _buildInfoText("SIG", "4/4"),
              ],
            );
          },
        ),
      ),
    );
  }

  Widget _buildInfoText(String label, String value) {
    return Column(
      mainAxisAlignment: MainAxisAlignment.center,
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Text(
          label,
          style: const TextStyle(
            color: Colors.grey,
            fontSize: 8,
            fontWeight: FontWeight.bold,
          ),
        ),
        Text(
          value,
          style: const TextStyle(
            color: Colors.lightGreenAccent,
            fontSize: 14,
            fontFamily: 'monospace',
          ),
        ),
      ],
    );
  }
}

class BpmControl extends ConsumerWidget {
  const BpmControl({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final bpm = ref.watch(karbeatStateProvider.select((s) => s.tempo));

    return FineGrainedInputWrapper<double>(
      value: bpm,
      min: 10.0,
      max: 999.0,
      step: 1.0,
      onChanged: (newBpm) {
        _updateBpm(ref, newBpm);
      },
      child: Listener(
        onPointerSignal: (event) {
          if (event is PointerScrollEvent) {
            final dy = event.scrollDelta.dy;
            final change = dy < 0 ? 0.1 : -0.1;
            _updateBpm(ref, bpm + change);
          }
        },
        child: GestureDetector(
          onVerticalDragUpdate: (details) {
            final change = details.primaryDelta! * -0.5;
            _updateBpm(ref, bpm + change);
          },
          child: MouseRegion(
            cursor: SystemMouseCursors.resizeUpDown,
            child: Container(
              color: Colors.transparent,
              child: Column(
                mainAxisAlignment: MainAxisAlignment.center,
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  const Text(
                    "BPM",
                    style: TextStyle(
                      color: Colors.grey,
                      fontSize: 8,
                      fontWeight: FontWeight.bold,
                    ),
                  ),
                  Text(
                    bpm.toStringAsFixed(1),
                    style: const TextStyle(
                      color: Colors.orangeAccent,
                      fontSize: 14,
                      fontFamily: 'monospace',
                    ),
                  ),
                ],
              ),
            ),
          ),
        ),
      ),
    );
  }

  void _updateBpm(WidgetRef ref, double newBpm) {
    final clamped = newBpm.clamp(10.0, 999.0);
    ref.read(karbeatStateProvider).setBpm(clamped);
  }
}
