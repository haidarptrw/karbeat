import 'dart:developer';
import 'dart:ui';

import 'package:flutter/material.dart';
import 'package:karbeat/src/rust/audio/event.dart';
import 'package:karbeat/state/app_state.dart';
import 'package:karbeat/utils/formatter.dart';
import 'package:karbeat/utils/scroll_behavior.dart';
import 'package:provider/provider.dart';

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

class DefaultControlPanel extends StatelessWidget {
  const DefaultControlPanel({super.key});

  @override
  Widget build(BuildContext context) {
    final builder = ControlPanelBuilder();

    // -- Navigation (Stateless) --
    builder.addItem(
      ControlPanelToolbarItem(
        name: "Tracks",
        icon: Icons.view_list,
        color: Colors.cyanAccent,
        onTap: () =>
            context.read<KarbeatState>().navigateTo(WorkspaceView.trackList),
        isActive: context.select<KarbeatState, bool>(
          (s) => s.currentView == WorkspaceView.trackList,
        ),
      ),
    );

    builder.addItem(
      ControlPanelToolbarItem(
        name: "Piano Roll",
        icon: Icons.piano,
        color: Colors.cyanAccent,
        onTap: () => context.read<KarbeatState>().navigateTo(WorkspaceView.pianoRoll),
        isActive: context.select<KarbeatState, bool>((s)=> s.currentView == WorkspaceView.pianoRoll)
      ),
    );

    builder.addItem(
      ControlPanelToolbarItem(
        name: "Mixer",
        icon: Icons.tune,
        color: Colors.cyanAccent,
        onTap: () => log("Nav to mixer"),
      ),
    );

    builder.addItem(
      ControlPanelToolbarItem(
        name: "Source",
        icon: Icons.group_work,
        color: Colors.cyanAccent,
        onTap: () =>
            context.read<KarbeatState>().navigateTo(WorkspaceView.source),
        isActive: context.select<KarbeatState, bool>(
          (s) => s.currentView == WorkspaceView.source,
        ),
      ),
    );

    builder.addDivider();

    // -- Transport --
    builder.addWidget(
      Selector<KarbeatState, bool>(
        selector: (_, state) => state.isPlaying,
        builder: (context, isPlaying, _) {
          return Row(
            mainAxisSize: MainAxisSize.min,
            children: [
              ControlPanelToolbarItem(
                name: isPlaying ? "Pause" : "Play",
                icon: isPlaying ? Icons.pause : Icons.play_arrow,
                color: Colors.greenAccent,
                isActive: isPlaying,
                onTap: () => context.read<KarbeatState>().togglePlay(),
              ),
              ControlPanelToolbarItem(
                name: "Stop",
                icon: Icons.stop,
                color: Colors.redAccent,
                onTap: () => context.read<KarbeatState>().stop(),
              ),
            ],
          );
        },
      ),
    );

    // -- Loop --
    builder.addWidget(
      Selector<KarbeatState, bool>(
        selector: (_, state) => state.isLooping,
        builder: (context, isLooping, _) {
          return ControlPanelToolbarItem(
            name: "Loop",
            icon: Icons.loop,
            color: Colors.orangeAccent,
            isActive: isLooping,
            onTap: () => context.read<KarbeatState>().toggleLoop(),
          );
        },
      ),
    );

    builder.addDivider();

    // -- Info Display --
    builder.addWidget(_buildInfoDisplay(context));

    builder.addDivider();

    // -- Tools --
    builder.addWidget(
      Selector<KarbeatState, ToolSelection>(
        selector: (_, state) => state.selectedTool,
        builder: (context, selectedTool, _) {
          return Row(
            mainAxisSize: MainAxisSize.min,
            children: [
              ControlPanelToolbarItem(
                name: "Select",
                icon: Icons.near_me,
                color: Colors.blueAccent,
                isActive: selectedTool == ToolSelection.pointer,
                onTap: () => context.read<KarbeatState>().selectTool(
                  ToolSelection.pointer,
                ),
              ),
              ControlPanelToolbarItem(
                name: "Cut",
                icon: Icons.content_cut,
                color: Colors.blueAccent,
                isActive: selectedTool == ToolSelection.cut,
                onTap: () =>
                    context.read<KarbeatState>().selectTool(ToolSelection.cut),
              ),
              ControlPanelToolbarItem(
                name: "Draw",
                icon: Icons.edit,
                color: Colors.blueAccent,
                isActive: selectedTool == ToolSelection.draw,
                onTap: () =>
                    context.read<KarbeatState>().selectTool(ToolSelection.draw),
              ),
              ControlPanelToolbarItem(
                name: "Move",
                icon: Icons.open_with,
                color: Colors.blueAccent,
                isActive: selectedTool == ToolSelection.move,
                onTap: () => context.read<KarbeatState>().selectTool(ToolSelection.move),
              ),
              ControlPanelToolbarItem(
                name: "Delete",
                icon: Icons.delete,
                color: Colors.red,
                isActive: selectedTool == ToolSelection.delete,
                onTap: () => context.read<KarbeatState>().selectTool(
                  ToolSelection.delete,
                ),
              ),
            ],
          );
        },
      ),
    );

    return builder.build();
  }

  Widget _buildInfoDisplay(BuildContext context) {
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 4),
      decoration: BoxDecoration(
        color: Colors.black54,
        borderRadius: BorderRadius.circular(4),
        border: Border.all(color: Colors.grey.shade700),
      ),
      child: IntrinsicHeight(
        child: StreamBuilder<PlaybackPosition>(
          stream: context.read<KarbeatState>().positionStream,
          builder: (context, asyncSnapshot) {
            final pos = asyncSnapshot.data;
            final bar = pos?.bar ?? 0;
            final beat = pos?.beat ?? 0;
            final samples = pos?.samples ?? 0;
            final bpm = pos?.tempo ?? 0.0;
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
                _buildInfoText("BPM", bpm.toStringAsFixed(1)),
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
