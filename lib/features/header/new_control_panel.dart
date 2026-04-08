// SOON-BE-IMPLEMENTED NEW CONTROL PANEL, which will be a floating pill tool selection widget
// that can be dragged around the screen.

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:karbeat/state/app_state.dart';

class FloatingPillControlPanel extends ConsumerWidget {
  final WorkspaceView currentScreen;
  final ToolSelection selectedTool;
  const FloatingPillControlPanel({
    super.key,
    required this.currentScreen,
    required this.selectedTool,
  });

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    return const Placeholder();
  }
}