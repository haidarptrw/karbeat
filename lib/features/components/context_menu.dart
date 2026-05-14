import 'package:flutter/material.dart';

/// Context action for an context menu available component
/// 
/// This is related to [ContextMenuWrapper].
class DawContextAction {
  final String title;
  final IconData? icon;
  final VoidCallback onTap;
  final bool isDestructive;
  final Color? color;

  DawContextAction({
    required this.title,
    required this.onTap,
    this.icon,
    this.isDestructive = false,
    this.color,
  });
}

/// A wrapper for a interactable widget that will display Context Menu
class ContextMenuWrapper extends StatelessWidget {
  final Widget child;
  final String? title;
  final Widget? header;
  final List<DawContextAction> actions;

  const ContextMenuWrapper({
    super.key,
    required this.child,
    required this.actions,
    this.title,
    this.header,
  });

  void _showContextMenu(BuildContext context) {
    showDialog(
      context: context,
      builder: (BuildContext dialogContext) {
        return AlertDialog(
          backgroundColor: Colors.grey.shade900,
          title: title != null 
              ? Text(title!, style: const TextStyle(color: Colors.white)) 
              : null,
          contentPadding: const EdgeInsets.only(top: 8.0, bottom: 8.0),
          content: Column(
            mainAxisSize: MainAxisSize.min,
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              // Header
              if (header != null) ...[
                Padding(
                  padding: const EdgeInsets.symmetric(horizontal: 24.0, vertical: 8.0),
                  child: header!,
                ),
                const Divider(color: Colors.white24, height: 16),
              ],
              
              // Actions list
              ...actions.map((action) {
                final color = action.isDestructive ? Colors.redAccent : Colors.white70;
                
                return ListTile(
                  contentPadding: const EdgeInsets.symmetric(horizontal: 24.0),
                  leading: action.icon != null
                      ? Icon(action.icon, color: color, size: 20)
                      : null,
                  title: Text(
                    action.title,
                    style: TextStyle(color: color, fontSize: 14),
                  ),
                  hoverColor: Colors.white10,
                  onTap: () {
                    Navigator.of(dialogContext).pop(); 
                    action.onTap(); 
                  },
                );
              }),
            ],
          ),
        );
      },
    );
  }

  @override
  Widget build(BuildContext context) {
    return GestureDetector(
      behavior: HitTestBehavior.opaque, 
      onLongPress: () => _showContextMenu(context), 
      onSecondaryTap: () => _showContextMenu(context), 
      child: child,
    );
  }
}

/// A floating panel that displays context-specific actions at the bottom of the screen.
/// Uses the shared [DawContextAction] model.
class FloatingContextPanel extends StatelessWidget {
  final List<DawContextAction> actions;
  final VoidCallback onClose;
  final String title;

  const FloatingContextPanel({
    super.key,
    required this.actions,
    required this.onClose,
    required this.title,
  });

  @override
  Widget build(BuildContext context) {
    if (actions.isEmpty) return const SizedBox.shrink();

    return Positioned(
      bottom: 20,
      left: 0,
      right: 0,
      child: Center(
        child: Material(
          elevation: 8,
          borderRadius: BorderRadius.circular(12),
          color: const Color(0xFF2A2A2A),
          child: Padding(
            padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 8),
            child: Row(
              mainAxisSize: MainAxisSize.min,
              children: [
                // Title / Close button
                InkWell(
                  onTap: onClose,
                  borderRadius: BorderRadius.circular(20),
                  child: Padding(
                    padding: const EdgeInsets.only(right: 12),
                    child: Row(
                      children: [
                        const Icon(Icons.close, color: Colors.white54, size: 16),
                        const SizedBox(width: 4),
                        Text(
                          title,
                          style: const TextStyle(
                            color: Colors.white54,
                            fontWeight: FontWeight.bold,
                            fontSize: 12,
                          ),
                        ),
                      ],
                    ),
                  ),
                ),
                Container(width: 1, height: 24, color: Colors.white24),
                const SizedBox(width: 8),

                // Action Buttons
                ...actions.map((action) => _FloatingActionButtonItem(action: action)),
              ],
            ),
          ),
        ),
      ),
    );
  }
}

class _FloatingActionButtonItem extends StatelessWidget {
  final DawContextAction action;

  const _FloatingActionButtonItem({required this.action});

  @override
  Widget build(BuildContext context) {
    // Resolve the final color based on destructive flag or explicit color
    final color = action.color ?? (action.isDestructive ? Colors.redAccent : Colors.white);
    
    return Tooltip(
      message: action.title,
      child: InkWell(
        onTap: action.onTap,
        borderRadius: BorderRadius.circular(8),
        child: Padding(
          padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 8),
          child: Column(
            mainAxisSize: MainAxisSize.min,
            children: [
              if (action.icon != null) ...[
                Icon(action.icon, color: color, size: 20),
                const SizedBox(height: 2),
              ],
              Text(
                action.title,
                style: TextStyle(color: color, fontSize: 10),
              ),
            ],
          ),
        ),
      ),
    );
  }
}
