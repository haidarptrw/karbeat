import 'package:flutter/material.dart';

/// Toolbar Menu Group
class KarbeatToolbarMenuGroup {
  final String id;
  final String title;
  final IconData icon;
  /// Define list of actions with type [KarbeatToolbarMenuAction]
  final List<KarbeatToolbarMenuAction> actions;

  KarbeatToolbarMenuGroup({
    required this.id,
    required this.title,
    required this.icon,
    required this.actions,
  });
}
/// Model for toolbar menu action
class KarbeatToolbarMenuAction {
  final String title;
  /// Shortcut for this action to give a context to the user. **Note that this shortcut only works on desktop app**
  final String? shortcut;
  /// Flag the menu if the menu is a dangerous operation
  final bool isDestructive;
  /// Define the callback when this action is executed
  final KarbeatToolbarMenuActionCallback? callback;

  KarbeatToolbarMenuAction(
    this.title, {
    this.shortcut,
    this.isDestructive = false,
    this.callback
  });
}

typedef KarbeatToolbarMenuActionCallback = void Function();

/// Factory for toolbar menu group
/// 
/// **DEVELOPER NOTE**: *Create a new initialization here for a new group menu type*
class KarbeatToolbarMenuGroupFactory {
  static KarbeatToolbarMenuGroup createProjectMenuGroup() =>
      KarbeatToolbarMenuGroup(
        id: "project",
        icon: Icons.work,
        title: "Project",
        actions: [
          KarbeatToolbarMenuAction('New project'),
          KarbeatToolbarMenuAction('Open project'),
          KarbeatToolbarMenuAction('Save Project'),
          KarbeatToolbarMenuAction('Save As...'),
          KarbeatToolbarMenuAction('Import Audio'),
          KarbeatToolbarMenuAction('Export Project'),
          KarbeatToolbarMenuAction('Settings'),
        ],
      );

  static KarbeatToolbarMenuGroup createEditMenuGroup() =>
      KarbeatToolbarMenuGroup(
        id: 'edit',
        icon: Icons.edit,
        title: 'Edit',
        actions: [
          KarbeatToolbarMenuAction('Undo', shortcut: 'Ctrl+Z'),
          KarbeatToolbarMenuAction('Redo', shortcut: 'CTRL+Y'),
        ],
      );

  static KarbeatToolbarMenuGroup createViewMenuGroup() =>
      KarbeatToolbarMenuGroup(
        id: 'view',
        title: 'View',
        icon: Icons.visibility,
        actions: [
          KarbeatToolbarMenuAction('Zoom in', shortcut: 'Ctrl+Plus'),
          KarbeatToolbarMenuAction('Zoom out', shortcut: 'CTRL+Minus'),

        ],
      );
}
