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
        boxShadow: [
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

  void addItem(ControlPanelToolbarItem item) {
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

  // Method to add non-standard items (like text displays)
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
    // Make the
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

class DefaultControlPanel extends ConsumerWidget {
  const DefaultControlPanel({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final state = ref.watch(karbeatStateProvider);
    final builder = ControlPanelBuilder();

    // Screen Navigation
    builder.addItem(
      ControlPanelToolbarItem(
        name: "Tracks",
        icon: Icons.view_list,
        color: Colors.cyanAccent,
        onTap: () =>
            ref.read(karbeatStateProvider).navigateTo(WorkspaceView.trackList),
        isActive: state.currentView == WorkspaceView.trackList,
      ),
    );

    builder.addItem(
      ControlPanelToolbarItem(
        name: "Piano Roll",
        icon: Icons.piano,
        color: Colors.cyanAccent,
        onTap: () =>
            ref.read(karbeatStateProvider).navigateTo(WorkspaceView.pianoRoll),
        isActive: state.currentView == WorkspaceView.pianoRoll,
      ),
    );

    builder.addItem(
      ControlPanelToolbarItem(
        name: "Mixer",
        icon: Icons.tune,
        color: Colors.cyanAccent,
        onTap: () =>
            ref.read(karbeatStateProvider).navigateTo(WorkspaceView.mixer),
        isActive: state.currentView == WorkspaceView.mixer,
      ),
    );

    builder.addItem(
      ControlPanelToolbarItem(
        name: "Source",
        icon: Icons.group_work,
        color: Colors.cyanAccent,
        onTap: () =>
            ref.read(karbeatStateProvider).navigateTo(WorkspaceView.source),
        isActive: state.currentView == WorkspaceView.source,
      ),
    );

    builder.addDivider();

    // Transport Panel
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
            name: "Snap to Grid", icon: Icons.grid_on, color: Colors.blueAccent,
            isActive: state.snapToGrid,
            onTap: () => ref.read(karbeatStateProvider).toggleSnapToGrid(),
          ),
        ],
      ),
    );
    builder.addDivider();

    // Info Display
    builder.addWidget(_buildInfoDisplay(context, ref));

    builder.addDivider();

    // Control Panel Tools
    builder.addWidget(
      Row(
        mainAxisSize: MainAxisSize.min,
        children: [
          ControlPanelToolbarItem(
            name: "Pointer",
            icon: Icons.near_me,
            color: Colors.blueAccent,
            isActive: state.selectedTool == ToolSelection.pointer,
            onTap: () => ref
                .read(karbeatStateProvider)
                .selectTool(ToolSelection.pointer),
          ),
          ControlPanelToolbarItem(
            name: "Cut",
            icon: Icons.content_cut,
            color: Colors.blueAccent,
            isActive: state.selectedTool == ToolSelection.cut,
            onTap: () =>
                ref.read(karbeatStateProvider).selectTool(ToolSelection.cut),
          ),
          ControlPanelToolbarItem(
            name: "Draw",
            icon: Icons.edit,
            color: Colors.blueAccent,
            isActive: state.selectedTool == ToolSelection.draw,
            onTap: () =>
                ref.read(karbeatStateProvider).selectTool(ToolSelection.draw),
          ),
          ControlPanelToolbarItem(
            name: "Move",
            icon: Icons.open_with,
            color: Colors.blueAccent,
            isActive: state.selectedTool == ToolSelection.move,
            onTap: () =>
                ref.read(karbeatStateProvider).selectTool(ToolSelection.move),
          ),
          ControlPanelToolbarItem(
            name: "Delete",
            icon: Icons.delete,
            color: Colors.red,
            isActive: state.selectedTool == ToolSelection.delete,
            onTap: () =>
                ref.read(karbeatStateProvider).selectTool(ToolSelection.delete),
          ),
          ControlPanelToolbarItem(
            name: "Range Select",
            icon: Icons.crop_free,
            color: Colors.blueAccent,
            isActive: state.selectedTool == ToolSelection.select,
            onTap: () =>
                ref.read(karbeatStateProvider).selectTool(ToolSelection.select),
          ),
          ControlPanelToolbarItem(
            name: "Resize",
            icon: Icons.zoom_out_map,
            color: Colors.blueAccent,
            isActive: state.selectedTool == ToolSelection.resize,
            onTap: () =>
                ref.read(karbeatStateProvider).selectTool(ToolSelection.resize),
          ),
        ],
      ),
    );

    return builder.build();
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
    // Watch the BPM state
    final bpm = ref.watch(karbeatStateProvider.select((s) => s.tempo));

    // 1. Wrap the entire control with the FineGrainedInputWrapper
    return FineGrainedInputWrapper<double>(
      value: bpm,
      min: 10.0,
      max: 999.0,
      step: 1.0, // For precise typing, jumping by 1.0 is standard
      onChanged: (newBpm) {
        // 2. Delegate the dialog's result directly to your state manager
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
          // Vertical drag does not conflict with the wrapper's longPress/secondaryTap
          onVerticalDragUpdate: (details) {
            final change = details.primaryDelta! * -0.5;
            _updateBpm(ref, bpm + change);
          },
          child: MouseRegion(
            cursor: SystemMouseCursors.resizeUpDown,
            // 3. Add a transparent container to ensure the entire region is clickable/draggable
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
