import 'dart:developer';

import 'package:flutter/material.dart';
import 'package:karbeat/features/header/control_panel.dart';
import 'package:karbeat/features/side_panel/side_panel.dart';
import 'package:karbeat/models/menu_group.dart';

class MainScreen extends StatefulWidget {
  const MainScreen({super.key});

  @override
  MainScreenState createState() {
    return MainScreenState();
  }
}

class MainScreenState extends State<MainScreen> {
  String? _openPanel;
  bool _isPlaying = false;
  bool _isLooping = false;
  String _selectedTool = 'Pointer';

  final List<KarbeatToolbarMenuGroup> _menuGroups = [
    KarbeatToolbarMenuGroupFactory.createProjectMenuGroup(),
    KarbeatToolbarMenuGroupFactory.createEditMenuGroup(),
    KarbeatToolbarMenuGroupFactory.createViewMenuGroup(),
  ];

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      backgroundColor: Colors.white,
      body: Stack(
        children: [
          Row(
            children: [
              _buildToolbar(),
              Expanded(child: _buildMainContent()),
            ],
          ),
          if (_openPanel != null)
            Positioned(
              left: 60, // Start after the toolbar
              top: 0,
              bottom: 0,
              child: _buildContextPanel(),
            ),
        ],
      ),
    );
  }

  void _togglePanel(String panelId) {
    setState(() {
      _openPanel = _openPanel == panelId ? null : panelId;
    });
  }

  void _executeAction(KarbeatToolbarMenuAction action) {
    setState(() {
      _openPanel = null;
    });

    action.callback?.call();
    ScaffoldMessenger.of(
      context,
    ).showSnackBar(SnackBar(content: Text('Executed: ${action.title}')));
  }

  Widget _buildToolbar() {
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
                  children: _menuGroups.map((group) {
                    return _buildToolbarItem(
                      icon: group.icon,
                      title: group.title,
                      isActive: _openPanel == group.id,
                      onTap: () => _togglePanel(group.id),
                    );
                  }).toList(),
                ),
              ),
            ),
          ),
        ],
      ),
    );
  }

  Widget _buildToolbarItem({
    required IconData icon,
    required String title,
    required bool isActive,
    required VoidCallback onTap,
  }) {
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

  Widget _buildContextPanel() {
    final group = _menuGroups.firstWhere((g) => g.id == _openPanel);
    return ContextPanel(
      group: group,
      onAction: _executeAction,
      onClose: () => setState(() => _openPanel = null),
    );
  }

  Widget _buildMainContent() {
    return Container(
      color: Colors.grey.shade800,
      child: Column(
        children: [
          SafeArea(
            top: true,
            bottom: false,
            child: Container(
              color: Colors.grey.shade50,
              child: _buildControlPanel(),
            ),
          ),
          Center(
            child: Column(
              mainAxisAlignment: MainAxisAlignment.center,
              children: [
                Text(
                  'Toolbar Test',
                  style: TextStyle(fontSize: 24, fontWeight: FontWeight.bold),
                ),
                SizedBox(height: 20),
                Text(
                  'Open Panel: ${_openPanel ?? "None"}',
                  style: TextStyle(fontSize: 16, color: Colors.grey),
                ),
              ],
            ),
          ),
        ],
      ),
    );
  }

  /// Build the control panel
  /// This panel includes things such as:
  ///   - Play, Pause, and Stop button
  ///   - Toggle Loop button
  ///   - Range selection mode
  ///   - Cut mode
  ///   - information like time elapsed and beat elapsed
  ///   - information of beat signature
  ///   - button to navigate to track list screen
  ///   - button to navigate to piano roll screen
  ///   - etc.
  Widget _buildControlPanel() {
    final builder = ControlPanelBuilder();

    // ==================== Section 2: Navigation =======================
    // Add track list menu
    builder.addItem(
      ControlPanelToolbarItem(
        name: "Tracks",
        icon: Icons.view_list,
        color: Colors.cyanAccent,
        onTap: () => log("Navigate to Track list"),
      ),
    );

    // Add piano roll menu
    builder.addItem(
      ControlPanelToolbarItem(
        name: "Piano Roll",
        icon: Icons.piano,
        color: Colors.cyanAccent,
        onTap: () => print("Nav to Piano Roll"),
      ),
    );

    builder.addDivider();

    // Section 2: ============== Transport =======================

    // add Toggle Play/Pause button
    builder.addItem(
      ControlPanelToolbarItem(
        name: _isPlaying ? "Pause" : "Play",
        icon: _isPlaying ? Icons.pause : Icons.play_arrow,
        color: Colors.greenAccent,
        isActive: _isPlaying,
        onTap: () => setState(() => _isPlaying = !_isPlaying),
      ),
    );

    // Add Stop button
    builder.addItem(
      ControlPanelToolbarItem(
        name: "Stop",
        icon: Icons.stop,
        color: Colors.redAccent,
        onTap: () => setState(() => _isPlaying = false),
      ),
    );

    builder.addItem(
      ControlPanelToolbarItem(
        name: "Loop",
        icon: Icons.loop,
        color: Colors.orangeAccent,
        isActive: _isLooping,
        onTap: () => setState(() => _isLooping = !_isLooping),
      ),
    );

    builder.addDivider();

    // Section 3: Information Display(Time/Beats)
    builder.addWidget(
      Container(
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
      ),
    );

    builder.addDivider();

    // ================ SECTION 4: Tools (Cut, Range, etc) ===================
    builder.addItem(
      ControlPanelToolbarItem(
        name: "Select",
        icon: Icons.near_me,
        color: Colors.blueAccent,
        isActive: _selectedTool == 'Pointer',
        onTap: () => setState(() {
          _selectedTool = 'Pointer';
        }),
      ),
    );
    builder.addItem(
      ControlPanelToolbarItem(
        name: "Cut",
        icon: Icons.content_cut,
        color: Colors.blueAccent,
        isActive: _selectedTool == 'Cut',
        onTap: () => setState(() => _selectedTool = 'Cut'),
      ),
    );
    builder.addItem(
      ControlPanelToolbarItem(
        name: "Draw",
        icon: Icons.edit,
        color: Colors.blueAccent,
        isActive: _selectedTool == 'Draw',
        onTap: () => setState(() => _selectedTool = 'Draw'),
      ),
    );

    return builder.build();
  }

  // Helper for the digital clock display style
  Widget _buildInfoText(String label, String value) {
    return Column(
      mainAxisAlignment: MainAxisAlignment.center,
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Text(
          label,
          style: TextStyle(
            color: Colors.grey,
            fontSize: 8,
            fontWeight: FontWeight.bold,
          ),
        ),
        Text(
          value,
          style: TextStyle(
            color: Colors.lightGreenAccent,
            fontSize: 14,
            fontFamily: 'monospace',
          ),
        ),
      ],
    );
  }
}
