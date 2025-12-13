import 'package:flutter/material.dart';
import 'package:karbeat/features/header/control_panel.dart';
import 'package:karbeat/features/screens/source_list_screen.dart';
import 'package:karbeat/features/screens/track_list_screen.dart';
import 'package:karbeat/state/app_state.dart';
import 'package:provider/provider.dart';

class MainContent extends StatelessWidget {
  const MainContent({super.key});

  @override
  Widget build(BuildContext context) {
    return Container(
      color: Colors.grey.shade800,
      child: Column(
        children: [
          SafeArea(
            top: true,
            bottom: false,
            child: Container(
              color: Colors.grey.shade50,
              child: const DefaultControlPanel(),
            ),
          ),
          Expanded(
            child: Selector<KarbeatState, WorkspaceView>(
              selector: (_, state) => state.currentView,
              builder: (context, currentView, _) {
                switch (currentView) {
                  case WorkspaceView.trackList:
                    return const TrackListScreen();
                  case WorkspaceView.source:
                    return const SourceListScreen();
                  default:
                    return const TrackListScreen();
                }
              },
            ),
          ),
        ],
      ),
    );
  }
}