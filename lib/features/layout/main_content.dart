import 'package:flutter/material.dart';
import 'package:karbeat/features/header/control_panel.dart';
import 'package:karbeat/features/screens/piano_roll_screen.dart';
import 'package:karbeat/features/screens/source_list_screen.dart';
import 'package:karbeat/features/screens/track_list_screen.dart';
import 'package:karbeat/src/rust/api/project.dart';
import 'package:karbeat/state/app_state.dart';
import 'package:karbeat/utils/logger.dart';
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
                  case WorkspaceView.pianoRoll:
                    return Selector<KarbeatState, (int?, int?)?>(
                      selector: (_, state) {
                        // 1. Get Selected IDs
                        final clipId = state.sessionState?.selectedClipId;
                        final trackId = state.sessionState?.selectedTrackId;

                        if (clipId == null || trackId == null) return null;

                        // 2. Find the Track
                        final track = state.tracks[trackId];
                        if (track == null) return null;

                        // 3. Find the Clip inside the Track
                        for (final clip in track.clips) {
                          if (clip.id == clipId) {
                            // 4. Check if it's MIDI and get Pattern ID
                            if (clip.source case UiClipSource_Midi(
                              :final patternId,
                            )) {
                              // Return Tuple: (PatternID, TrackID)
                              return (patternId, trackId);
                            }
                            return null; // Selected clip is not MIDI (e.g. Audio)
                          }
                        }
                        return null; // Clip not found in track
                      },
                      builder: (context, selectionData, _) {
                        final patternId = selectionData?.$1;
                        final trackId = selectionData?.$2;

                        KarbeatLogger.info(
                          "Opening piano roll for pattern: $patternId on track: $trackId",
                        );

                        return PianoRollScreen(
                          patternId: patternId,
                          parentTrackId: trackId,
                        );
                      },
                    );
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
