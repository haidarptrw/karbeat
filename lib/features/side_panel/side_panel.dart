import 'package:flutter/material.dart';
import 'package:karbeat/models/menu_group.dart';

class ContextPanel extends StatelessWidget {
  final KarbeatToolbarMenuGroup group;
  final Function(KarbeatToolbarMenuAction) onAction;
  final VoidCallback onClose;

  const ContextPanel({
    super.key,
    required this.group,
    required this.onAction,
    required this.onClose,
  });

 @override
  Widget build(BuildContext context) {
    return Container(
      width: 240,
      color: Colors.white,
      child: Column(
        children: [
          // Header
          Container(
            height: 50,
            color: Colors.purple.shade700,
            padding: EdgeInsets.symmetric(horizontal: 16),
            child: Row(
              children: [
                Text(
                  group.title,
                  style: TextStyle(
                    color: Colors.white,
                    fontWeight: FontWeight.bold,
                  ),
                ),
                Spacer(),
                IconButton(
                  icon: Icon(Icons.close, color: Colors.white, size: 18),
                  onPressed: onClose,
                ),
              ],
            ),
          ),
          
          // Simple text list
          Expanded(
            child: ListView(
              padding: EdgeInsets.all(8),
              children: group.actions.map((action) {
                return Material(
                  color: Colors.transparent,
                  child: InkWell(
                    onTap: () => onAction(action),
                    borderRadius: BorderRadius.circular(4),
                    child: Container(
                      padding: EdgeInsets.symmetric(horizontal: 16, vertical: 12),
                      child: Row(
                        children: [
                          Expanded(
                            child: Text(
                              action.title,
                              style: TextStyle(
                                fontSize: 14,
                                color: action.isDestructive ? Colors.red : Colors.grey.shade800,
                              ),
                            ),
                          ),
                          if (action.shortcut != null)
                            Container(
                              margin: EdgeInsets.only(left: 8),
                              padding: EdgeInsets.symmetric(horizontal: 6, vertical: 2),
                              decoration: BoxDecoration(
                                color: Colors.grey.shade200,
                                borderRadius: BorderRadius.circular(3),
                              ),
                              child: Text(
                                action.shortcut!,
                                style: TextStyle(
                                  fontSize: 10,
                                  color: Colors.grey.shade700,
                                  fontFamily: 'Monospace',
                                ),
                              ),
                            ),
                        ],
                      ),
                    ),
                  ),
                );
              }).toList(),
            ),
          ),
        ],
      ),
    );
  }
}