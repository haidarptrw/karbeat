import 'dart:developer';

import 'package:flutter/material.dart';
import 'package:karbeat/features/header/control_panel.dart';
import 'package:karbeat/features/screens/source_list_screen.dart';
import 'package:karbeat/features/screens/track_list_screen.dart';
import 'package:karbeat/features/side_panel/side_panel.dart';
import 'package:karbeat/state/app_state.dart';
import 'package:provider/provider.dart';

class MainScreen extends StatelessWidget {
  const MainScreen({super.key});

  @override
  Widget build(BuildContext context) {
    // REMOVED: Top-level Consumer.
    // The Scaffold is static; only specific children need to rebuild.
    return Scaffold(
      backgroundColor: Colors.white,
      body: Stack(
        children: [
          Row(
            children: [
              const _SidebarToolbar(), // Extracted to a const widget
              Expanded(
                child: _MainContent(),
              ), // Extracted to keep MainScreen clean
            ],
          ),
          // Optimized Context Panel Overlay
          // Only rebuilds if the specific panel ID changes
          Selector<KarbeatState, ToolbarMenuContextGroup>(
            selector: (_, state) => state.currentToolbarContext,
            builder: (context, currentContext, child) {
              if (currentContext == ToolbarMenuContextGroup.none) {
                return const SizedBox.shrink();
              }
              return Positioned(
                left: 60,
                top: 0,
                bottom: 0,
                child: _buildContextPanel(context, currentContext),
              );
            },
          ),
        ],
      ),
    );
  }

  Widget _buildContextPanel(
    BuildContext context,
    ToolbarMenuContextGroup currentContext,
  ) {
    // Note: We don't need a Consumer here because we passed the currentContext in
    final group = KarbeatState.menuGroups.firstWhere(
      (g) => g.id == currentContext,
    );

    return ContextPanel(
      group: group,
      onAction: (action) {
        // Read context.read to avoid listening
        final state = context.read<KarbeatState>();
        state.closeContextPanel();
        action.callback?.call(context, state);
        ScaffoldMessenger.of(
          context,
        ).showSnackBar(SnackBar(content: Text('Executed: ${action.title}')));
      },
      onClose: () => context.read<KarbeatState>().closeContextPanel(),
    );
  }
}

// =============================================================================
// 1. OPTIMIZED SIDEBAR (TOOLBAR)
// =============================================================================
class _SidebarToolbar extends StatelessWidget {
  const _SidebarToolbar();

  @override
  Widget build(BuildContext context) {
    return Container(
      width: 60,
      color: Colors.grey.shade900,
      child: Column(
        children: [
          Expanded(
            child: Container(
              color: Colors.grey.shade900,
              child: SingleChildScrollView(
                child: Column(
                  mainAxisAlignment: MainAxisAlignment.start,
                  // Selector ensures this list only rebuilds when a menu opens/closes
                  children: [
                    Selector<KarbeatState, ToolbarMenuContextGroup>(
                      selector: (_, state) => state.currentToolbarContext,
                      builder: (context, currentContext, _) {
                        return Column(
                          children: KarbeatState.menuGroups.map((group) {
                            return _SidebarItem(
                              icon: group.icon,
                              title: group.title,
                              isActive: currentContext == group.id,
                              onTap: () => context
                                  .read<KarbeatState>()
                                  .toggleToolbarContext(group.id),
                            );
                          }).toList(),
                        );
                      },
                    ),
                  ],
                ),
              ),
            ),
          ),
        ],
      ),
    );
  }
}

class _SidebarItem extends StatelessWidget {
  final IconData icon;
  final String title;
  final bool isActive;
  final VoidCallback onTap;

  const _SidebarItem({
    required this.icon,
    required this.title,
    required this.isActive,
    required this.onTap,
  });

  @override
  Widget build(BuildContext context) {
    return Container(
      width: 60,
      height: 60,
      decoration: BoxDecoration(
        color: isActive ? Colors.purple.shade700 : Colors.transparent,
        border: isActive
            ? Border(left: BorderSide(color: Colors.purple.shade300, width: 3))
            : null,
      ),
      child: Tooltip(
        message: title,
        child: IconButton(
          icon: Icon(
            icon,
            color: isActive ? Colors.white : Colors.grey.shade400,
            size: 22,
          ),
          onPressed: onTap,
        ),
      ),
    );
  }
}

// =============================================================================
// 2. OPTIMIZED MAIN CONTENT & HEADER
// =============================================================================
class _MainContent extends StatelessWidget {
  @override
  Widget build(BuildContext context) {
    return Container(
      color: Colors.grey.shade800,
      child: Column(
        children: [
          SafeArea(
            top: true,
            bottom: false,
            child: Container(
              color: Colors.grey.shade50,
              child: const _ControlPanel(), // Extracted widget
            ),
          ),
          Expanded(
            child: Selector<KarbeatState, WorkspaceView>(
              selector: (_, state) => state.currentView,
              builder: (context, currentView, _) {
                switch (currentView) {
                  case WorkspaceView.trackList:
                    return TrackListScreen();
                  case WorkspaceView.source:
                    return SourceListScreen(); // The new screen
                  // case WorkspaceView.pianoRoll:
                  //   return const Center(child: Text("Piano Roll (TODO)", style: TextStyle(color: Colors.white)));
                  // case WorkspaceView.mixer:
                  //   return const Center(child: Text("Mixer (TODO)", style: TextStyle(color: Colors.white)));
                  default:
                    return TrackListScreen(); // for now fallback to TrackListScreen
                }
              },
            ),
          ),
        ],
      ),
    );
  }
}

// =============================================================================
// 3. OPTIMIZED CONTROL PANEL (GRANULAR REBUILDS)
// =============================================================================
class _ControlPanel extends StatelessWidget {
  const _ControlPanel();

  @override
  Widget build(BuildContext context) {
    final builder = ControlPanelBuilder();

    // -- Navigation (Stateless) --
    builder.addItem(
      ControlPanelToolbarItem(
        name: "Tracks",
        icon: Icons.view_list,
        color: Colors.cyanAccent,
        onTap: () => context.read<KarbeatState>().navigateTo(WorkspaceView.trackList),
        isActive: context.select<KarbeatState, bool>((s) => s.currentView == WorkspaceView.trackList),
      ),
    );

    builder.addItem(
      ControlPanelToolbarItem(
        name: "Piano Roll",
        icon: Icons.piano,
        color: Colors.cyanAccent,
        onTap: () => log("Nav to Piano Roll"),
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
        onTap: () => context.read<KarbeatState>().navigateTo(WorkspaceView.source),
        isActive: context.select<KarbeatState, bool>((s) => s.currentView == WorkspaceView.source),
      ),
    );

    builder.addDivider();

    // -- Transport (Listens to isPlaying) --
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

    // -- Loop (Listens to isLooping) --
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

    // -- Info Display (Static for now) --
    builder.addWidget(_buildInfoDisplay());

    builder.addDivider();

    // -- Tools (Listens to selectedTool) --
    // We wrap the *Group* of tools in one Selector.
    // Alternatively, wrap each item if you want extreme optimization,
    // but wrapping the group is usually sufficient.
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
            ],
          );
        },
      ),
    );

    return builder.build();
  }

  Widget _buildInfoDisplay() {
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 4),
      decoration: BoxDecoration(
        color: Colors.black54,
        borderRadius: BorderRadius.circular(4),
        border: Border.all(color: Colors.grey.shade700),
      ),
      child: IntrinsicHeight(
        child: Row(
          children: [
            _buildInfoText("BAR", "004"),
            const SizedBox(width: 10),
            _buildInfoText("BEAT", "02"),
            const VerticalDivider(color: Colors.grey, width: 20),
            _buildInfoText("TIME", "00:08:45"),
            const VerticalDivider(color: Colors.grey, width: 20),
            _buildInfoText("BPM", "67"),
            const VerticalDivider(color: Colors.grey, width: 20),
            _buildInfoText("SIG", "4/4"),
          ],
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
