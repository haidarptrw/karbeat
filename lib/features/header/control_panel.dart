import 'dart:ui';

import 'package:flutter/material.dart';

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
        behavior: _DragScrollBehavior(),
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

// Custom Behavior to allow Mouse Dragging (Standard 'Inverse' Scrolling)
class _DragScrollBehavior extends MaterialScrollBehavior {
  @override
  Set<PointerDeviceKind> get dragDevices => {
    PointerDeviceKind.touch,
    PointerDeviceKind.mouse,
    PointerDeviceKind.trackpad,
  };
}
