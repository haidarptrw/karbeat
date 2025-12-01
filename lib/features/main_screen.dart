import 'package:flutter/material.dart';
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
      color: Colors.white,
      child: Center(
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
    );
  }
}
