import 'dart:developer';

import 'package:flutter/foundation.dart';
import 'package:karbeat/models/menu_group.dart';
import 'package:karbeat/src/rust/api/audio.dart';
import 'package:karbeat/src/rust/api/project.dart';
import 'package:karbeat/src/rust/api/track.dart';
import 'package:karbeat/src/rust/api/transport.dart';
import 'package:karbeat/src/rust/core/project.dart';

enum ToolSelection { pointer, cut, draw }

enum WorkspaceView { trackList, pianoRoll, mixer, source }

enum ToolbarMenuContextGroup { none, project, edit, view }

class KarbeatState extends ChangeNotifier {
  TransportState _transportState = TransportState(
    isPlaying: false,
    isRecording: false,
    isLooping: false,
    playheadPositionSamples: 0,
    loopStartSamples: 0,
    loopEndSamples: 0,
    bpm: 67.0,
    timeSignature: (4, 4),
    barTracker: 0,
    beatTracker: 0,
  );

  ProjectMetadata _metadata = ProjectMetadata(
    name: "Untitled",
    author: "User",
    version: "1.0.0",
    createdAt: 0, // Assuming u64
  );

  AudioHardwareConfig _hardwareConfig = AudioHardwareConfig(
    selectedInputDevice: '',
    selectedOutputDevice: '',
    sampleRate: 44100,
    bufferSize: 256,
    cpuLoad: 0,
  );

  Map<int, UiTrack> _tracks = {};
  Map<int, AudioWaveformUiForSourceList> _audioSources = {};

  static final List<KarbeatToolbarMenuGroup> menuGroups = [
    KarbeatToolbarMenuGroupFactory.createProjectMenuGroup(),
    KarbeatToolbarMenuGroupFactory.createEditMenuGroup(),
    KarbeatToolbarMenuGroupFactory.createViewMenuGroup(),
  ];

  // =========== EDITOR STATE ====================
  ToolSelection _selectedTool = ToolSelection.pointer;
  WorkspaceView _currentView = WorkspaceView.trackList;
  ToolbarMenuContextGroup _currentToolbarContext = ToolbarMenuContextGroup.none;

  /// Denominator of the grid size (e.g 4 = 1/4 note, 16 = 1/16 note)
  int gridSize = 4;

  // ============== GETTERS =================
  TransportState get transport => _transportState;
  ProjectMetadata get metadata => _metadata;
  bool get isPlaying => _transportState.isPlaying;
  bool get isLooping => _transportState.isLooping;
  double get tempo => _transportState.bpm;
  Map<int, UiTrack> get tracks => _tracks;
  Map<int, AudioWaveformUiForSourceList> get audioSources => _audioSources;
  ToolSelection get selectedTool => _selectedTool;
  WorkspaceView get currentView => _currentView;
  ToolbarMenuContextGroup get currentToolbarContext => _currentToolbarContext;
  AudioHardwareConfig get hardwareConfig => _hardwareConfig;

  // =============== GLOBAL UI STATE ==========================
  double horizontalZoomLevel = 1000;
  Map<int, int> trackIdHeightMap = {};

   // =============== PLACEMENT MODE STATE =====================
  int? _placingSourceId; // The ID of the source we are moving
  int? get placingSourceId => _placingSourceId;
  
  // Track where the user wants to drop it
  double _placementTimeSamples = 0.0;
  int _placementTrackId = -1;

  bool get isPlacing => _placingSourceId != null;


  // ================ SYNCHRONIZATION ======================
  // / Syncs the core project structure (Tracks, Metadata)
  // / Call this when: Loading project, Adding tracks, changing BPM
  // Future<void> syncProjectState() async {
  //   // Call Rust API (get_ui_state returns Option<UiProjectState>)

  //   final newState = await getUiState();

  //   if (newState != null) {
  //     _metadata = newState.metadata;
  //     _tracks = newState.tracks;
  //     notifyListeners();
  //   }
  // }

  /// Syncs only the track state
  Future<void> syncTrackState() async {
    try {
      final newState = await getTracks();
      _tracks = newState;
      notifyListeners();
    } catch (e) {
      log("error when syncing the track state: $e");
    }
  }

  /// Syncs only the transport (Playhead, Play state)
  /// Call this inside a Ticker (e.g. 60Hz)
  Future<void> syncTransportState() async {
    try {
      final newState = await getTransportState();
      // Optimization: Only notify if changed significantly (optional)
      _transportState = newState;
      notifyListeners();
    } catch (e) {
      log("Transport sync failed: $e");
    }
  }

  /// Syncs only the metadata (Project name, BPM, Time signature)
  Future<void> syncMetadataState() async {
    try {
      final newState = await getProjectMetadata();
      _metadata = newState;
      notifyListeners();
    } catch (e) {
      log("failed when syncing metadata: $e");
    }
  }

  /// Syncs the list of loaded audio files
  /// Call this when: Adding a file, Removing a file
  Future<void> syncSourceList() async {
    final sources = await getSourceList();
    if (sources != null) {
      _audioSources = Map.from(sources);
      notifyListeners();
    }
  }

  Future<void> syncAudioHardwareConfigState() async {
    try {
      final newState = await getAudioConfig();
      _hardwareConfig = newState;
      notifyListeners();
    } catch (e) {
      log("Failed to sync audio hardware state: $e");
    }
  }


  // =============== ACTIONS ===============

  /// Loads an audio file and refreshes the source list
  Future<void> addAudioFile(String path) async {
    await addAudioSource(filePath: path);
    await syncSourceList();
  }

  Future<void> addTrack(TrackType type) async {
    try {
      await addNewTrack(trackType: type);
      await syncTrackState();
    } catch (e) {
      log("Failed to add track: $e");
    }
  }

  Future<void> togglePlay() async {
    try {
      final newPlaying = !_transportState.isPlaying;
      await setPlaying(val: newPlaying);
      await syncTransportState();
    } catch (e) {
      log("Failed to toggle play: $e");
    }
  }

  Future<void> stop() async {
    try {
      await setPlaying(val: false);
      await setPlayhead(val: 0);
      await syncTransportState();
    } catch (e) {
      log("Failed to stop play: $e");
    }
  }

  Future<void> toggleLoop() async {
    try {
      final newLooping = !_transportState.isLooping;
      await setLooping(val: newLooping);
      await syncTransportState();
    } catch (e) {
      log("Failed to toggle loop: $e");
    }
  }

  void selectTool(ToolSelection tool) {
    if (_selectedTool != tool) {
      _selectedTool = tool;
      notifyListeners();
    }
  }

  void toggleToolbarContext(ToolbarMenuContextGroup group) {
    if (group == _currentToolbarContext) {
      // Toggle off
      _currentToolbarContext = ToolbarMenuContextGroup.none;
    } else {
      _currentToolbarContext = group;
    }
    notifyListeners();
  }

  void closeContextPanel() {
    _currentToolbarContext = ToolbarMenuContextGroup.none;
    notifyListeners();
  }

  void navigateTo(WorkspaceView view) {
    if (_currentView != view) {
      _currentView = view;
      notifyListeners();
    }
  }

  void setGridSize(int newSize) {
    if (gridSize != newSize) {
      gridSize = newSize;
      notifyListeners();
    }
  }

    // ============= PLACEMENT MODE LOGIC =================
  
  void startPlacement(int sourceId) {
    _placingSourceId = sourceId;
    // Switch view to track list immediately so user can place it
    navigateTo(WorkspaceView.trackList);
    notifyListeners();
  }

  /// Updates the target location without notifying all listeners
  /// (Use setState in the UI for visual feedback to avoid global rebuilds)
  void updatePlacementTarget(int trackId, double timeSamples) {
    _placementTrackId = trackId;
    _placementTimeSamples = timeSamples;
  }

  Future<void> confirmPlacement() async {
    if (_placingSourceId != null && _placementTrackId != -1) {
      try {
        await createClip(
          sourceId: _placingSourceId!,
          sourceType: UiSourceType.audio, // Assuming Audio for now
          trackId: _placementTrackId,
          startTime: _placementTimeSamples.toInt(),
        );
        
        // Refresh tracks to see the new clip
        await syncTrackState();
        
        // Reset mode
        _placingSourceId = null;
        _placementTrackId = -1;
        notifyListeners();
        
      } catch (e) {
        log("Error creating clip: $e");
        // Optionally show error to user via a global key or snackbar service
      }
    }
  }

  void cancelPlacement() {
    _placingSourceId = null;
    _placementTrackId = -1;
    notifyListeners();
  }
}
