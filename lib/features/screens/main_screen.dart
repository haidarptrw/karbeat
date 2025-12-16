import 'package:flutter/material.dart';
import 'package:karbeat/features/layout/main_content.dart';
import 'package:karbeat/features/side_panel/side_panel.dart';
import 'package:karbeat/features/side_panel/sidebar.dart';
import 'package:karbeat/state/app_state.dart';
import 'package:provider/provider.dart';

class MainScreen extends StatelessWidget {
  const MainScreen({super.key});

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      backgroundColor: Colors.white,
      body: Stack(
        children: [
          const Row(
            children: [
              Sidebar(),
              Expanded(
                child: MainContent(),
              ),
            ],
          ),
          // Optimized Context Panel Overlay
          Selector<KarbeatState, ToolbarMenuContextGroup>(
            selector: (_, state) => state.currentToolbarContext,
            builder: (context, currentContext, child) {
              if (currentContext == ToolbarMenuContextGroup.none) {
                return const SizedBox.shrink();
              }
              return Positioned(
                left: 60,
                top: 0,
                bottom: 0,
                child: _buildContextPanel(context, currentContext),
              );
            },
          ),
        ],
      ),
    );
  }

  Widget _buildContextPanel(
    BuildContext context,
    ToolbarMenuContextGroup currentContext,
  ) {
    final group = KarbeatState.menuGroups.firstWhere(
      (g) => g.id == currentContext,
    );

    return ContextPanel(
      group: group,
      onAction: (action) {
        final state = context.read<KarbeatState>();
        state.closeContextPanel();
        action.callback?.call(context, state);
        ScaffoldMessenger.of(
          context,
        ).showSnackBar(SnackBar(content: Text('Executed: ${action.title}')));
      },
      onClose: () => context.read<KarbeatState>().closeContextPanel(),
    );
  }
}