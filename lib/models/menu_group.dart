import 'dart:io' show Platform;

import 'package:flutter/material.dart';
import 'package:karbeat/state/app_state.dart';
import 'package:file_picker/file_picker.dart';
import 'package:window_manager/window_manager.dart';

/// Toolbar Menu Group
class KarbeatToolbarMenuGroup {
  final ToolbarMenuContextGroup id;
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
    this.callback,
  });
}

typedef KarbeatToolbarMenuActionCallback =
    void Function(BuildContext, KarbeatState);

/// Factory for toolbar menu group
///
/// **DEVELOPER NOTE**: *Create a new initialization here for a new group menu type*
class KarbeatToolbarMenuGroupFactory {
  /// Helper to safely update the window title on Desktop platforms
  static Future<void> _updateWindowTitle(String filePath) async {
    // Only attempt to change the window title on desktop OS
    if (Platform.isWindows || Platform.isMacOS || Platform.isLinux) {
      // Extract just the file name from the full path (e.g., "MySong.karbeat")
      final fileName = filePath.split(RegExp(r'[/\\]')).last;

      try {
        await windowManager.setTitle('Karbeat — $fileName');
      } catch (e) {
        debugPrint("Failed to set window title: $e");
      }
    }
  }

  /// Helper to handle "Save As" logic used by both Save and Save As buttons
  static Future<void> _performSaveAs(
    BuildContext context,
    KarbeatState state,
  ) async {
    final path = await FilePicker.platform.saveFile(
      dialogTitle: 'Save Project As...',
      fileName: 'untitled.karbeat',
      type: FileType.custom,
      allowedExtensions: ['karbeat'],
    );

    if (path != null) {
      if (context.mounted) {
        showDialog(
          context: context,
          barrierDismissible: false,
          builder: (context) =>
              const Center(child: CircularProgressIndicator()),
        );
      }

      await state.saveProject(path);

      // Update the active file path and window title after successful save
      state.currentFilePath = path;
      await _updateWindowTitle(path);

      if (context.mounted) Navigator.of(context).pop();
    }
  }

  static KarbeatToolbarMenuGroup
  createProjectMenuGroup() => KarbeatToolbarMenuGroup(
    id: ToolbarMenuContextGroup.project,
    icon: Icons.work,
    title: "Project",
    actions: [
      KarbeatToolbarMenuAction('New project'),
      KarbeatToolbarMenuAction(
        'Open project',
        shortcut: 'Ctrl+O',
        callback: (context, state) async {
          final result = await FilePicker.platform.pickFiles(
            type: FileType.custom,
            allowedExtensions: ['karbeat'],
          );
          if (result != null && result.files.single.path != null) {
            final path = result.files.single.path!;
            if (context.mounted) {
              showDialog(
                context: context,
                barrierDismissible: false,
                builder: (context) =>
                    const Center(child: CircularProgressIndicator()),
              );
            }

            await state.loadProject(path);

            // Update the active file path and window title after successful load
            state.currentFilePath = path;
            await _updateWindowTitle(path);

            if (context.mounted) Navigator.of(context).pop();
          }
        },
      ),
      KarbeatToolbarMenuAction(
        'Save Project',
        shortcut: 'Ctrl+S',
        callback: (context, state) async {
          if (state.currentFilePath == null) {
            // If the project has never been saved, trigger Save As
            await _performSaveAs(context, state);
          } else {
            // Otherwise, save silently to the existing path
            showDialog(
              context: context,
              barrierDismissible: false,
              builder: (context) =>
                  const Center(child: CircularProgressIndicator()),
            );

            await state.saveProject(state.currentFilePath!);

            if (context.mounted) Navigator.of(context).pop();
          }
        },
      ),
      KarbeatToolbarMenuAction(
        'Save As...',
        shortcut: 'Ctrl+Shift+S',
        callback: (context, state) async {
          await _performSaveAs(context, state);
        },
      ),
      KarbeatToolbarMenuAction('Import Audio'),
      KarbeatToolbarMenuAction('Export Project'),
      KarbeatToolbarMenuAction('Settings'),
    ],
  );

  static KarbeatToolbarMenuGroup createEditMenuGroup() =>
      KarbeatToolbarMenuGroup(
        id: ToolbarMenuContextGroup.edit,
        icon: Icons.edit,
        title: 'Edit',
        actions: [
          KarbeatToolbarMenuAction('Undo', shortcut: 'Ctrl+Z'),
          KarbeatToolbarMenuAction('Redo', shortcut: 'CTRL+Y'),
        ],
      );

  static KarbeatToolbarMenuGroup createViewMenuGroup() =>
      KarbeatToolbarMenuGroup(
        id: ToolbarMenuContextGroup.view,
        title: 'View',
        icon: Icons.visibility,
        actions: [
          KarbeatToolbarMenuAction('Zoom in', shortcut: 'Ctrl+Plus'),
          KarbeatToolbarMenuAction('Zoom out', shortcut: 'CTRL+Minus'),
        ],
      );
}
