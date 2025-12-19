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
                    return Selector<KarbeatState, int?>(
                      selector: (_, state) {
                        // Get Selected Clip ID
                        final clipId = state.sessionState?.selectedClipId;
                        final trackId = state.sessionState?.selectedTrackId;
                        if (clipId == null || trackId == null) return null;

                        // Find the Clip object (Iterate tracks)
                        final track = state.tracks[trackId];
                        if (track == null) {
                          for (final track in state.tracks.values) {
                            for (final clip in track.clips) {
                              if (clip.id == clipId) {
                                if (clip.source case UiClipSource_Midi(
                                  :final patternId,
                                )) {
                                  return patternId;
                                }
                                // Selected clip exists but isn't MIDI (e.g. Audio)
                                return null;
                              }
                            }
                          }
                        } else {
                          for (final clip in track.clips) {
                            if (clip.id == clipId) {
                              if (clip.source case UiClipSource_Midi(
                                :final patternId,
                              )) {
                                return patternId;
                              }
                              // Selected clip exists but isn't MIDI (e.g. Audio)
                              return null;
                            }
                          }
                        }

                        return null; // Clip not found
                      },
                      builder: (context, patternId, _) {
                        KarbeatLogger.info("Opening piano roll for pattern id $patternId");
                        return PianoRollScreen(patternId: patternId);
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
