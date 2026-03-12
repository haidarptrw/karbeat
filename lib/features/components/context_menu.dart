import 'package:flutter/material.dart';

/// Context action for an context menu available component
/// 
/// This is related to [ContextMenuWrapper].
class KarbeatContextAction {
  final String title;
  final IconData? icon;
  final VoidCallback onTap;
  final bool isDestructive;

  KarbeatContextAction({
    required this.title,
    required this.onTap,
    this.icon,
    this.isDestructive = false,
  });
}

/// A wrapper for a interactable widget that will display Context Menu
class ContextMenuWrapper extends StatelessWidget {
  final Widget child;
  final String? title;
  final Widget? header;
  final List<KarbeatContextAction> actions;

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