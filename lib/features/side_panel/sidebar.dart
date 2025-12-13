import 'package:flutter/material.dart';
import 'package:karbeat/models/menu_group.dart';
import 'package:karbeat/state/app_state.dart';
import 'package:provider/provider.dart';

class Sidebar extends StatelessWidget {
  const Sidebar({super.key});

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
                  children: [
                    Selector<KarbeatState, ToolbarMenuContextGroup>(
                      selector: (_, state) => state.currentToolbarContext,
                      builder: (context, currentContext, _) {
                        return Column(
                          children: KarbeatState.menuGroups.map((group) {
                            return SidebarItem(
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

class SidebarItem extends StatelessWidget {
  final IconData icon;
  final String title;
  final bool isActive;
  final VoidCallback onTap;

  const SidebarItem({
    super.key,
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
