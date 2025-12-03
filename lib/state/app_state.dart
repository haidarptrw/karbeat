import 'package:flutter/foundation.dart';
import 'package:karbeat/models/menu_group.dart';
import 'package:karbeat/src/rust/api/project.dart';
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
  );

  ProjectMetadata _metadata = ProjectMetadata(
    name: "Untitled",
    author: "User",
    version: "1.0.0",
    createdAt: 0, // Assuming u64
    bpm: 120.0,
    timeSignature: (4, 4),
  );

  List<UiTrack> _tracks = [];
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

  // ============== GETTERS =================
  TransportState get transport => _transportState;
  ProjectMetadata get metadata => _metadata;
  bool get isPlaying => _transportState.isPlaying;
  bool get isLooping => _transportState.isLooping;
  double get tempo => _metadata.bpm;
  List<UiTrack> get tracks => _tracks;
  Map<int, AudioWaveformUiForSourceList> get audioSources => _audioSources;
  ToolSelection get selectedTool => _selectedTool;
  WorkspaceView get currentView => _currentView;
  ToolbarMenuContextGroup get currentToolbarContext => _currentToolbarContext;

  // =============== GLOBAL UI STATE ==========================
  double horizontalZoomLevel = 10;
  Map<int, int> trackIdHeightMap = {};

  // ================ SYNCHRONIZATION ======================
  /// Syncs the core project structure (Tracks, Metadata)
  /// Call this when: Loading project, Adding tracks, changing BPM
  Future<void> syncProjectState() async {
    // Call Rust API (get_ui_state returns Option<UiProjectState>)
    final newState = await getUiState();

    if (newState != null) {
      _metadata = newState.metadata;
      _tracks = newState.tracks;
      notifyListeners();
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
      print("Transport sync failed: $e");
    }
  }

  /// Syncs the list of loaded audio files
  /// Call this when: Adding a file, Removing a file
  Future<void> syncSourceList() async {
    final sources = await getSourceList();
    if (sources != null) {
      // Rust returns HashMap<u32, ...>, Dart converts to Map<int, ...>
      _audioSources = Map.from(sources);
      notifyListeners();
    }
  }

  // =============== ACTIONS ===============
  /// Loads an audio file and refreshes the source list
  Future<void> addAudioFile(String path) async {
    // 1. Call Rust to load
    await addAudioSource(filePath: path);

    // 2. Refresh List
    await syncSourceList();
  }

  void togglePlay() {
    // Optimistic UI Update (Immediate feedback)
    final newPlaying = !_transportState.isPlaying;
    _transportState = _transportState.copyWith(isPlaying: newPlaying);
    notifyListeners();

    // Send Command to Rust (Assuming you have setPlaying exposed)
    setPlaying(val: newPlaying);
  }

  void stop() {
    _transportState = _transportState.copyWith(
      isPlaying: false,
      playheadPositionSamples: 0,
    );
    notifyListeners();

    setPlaying(val: false);
    setPlayhead(val: 0);
  }

  void toggleLoop() {
    final newLooping = !_transportState.isLooping;
    _transportState = _transportState.copyWith(isLooping: newLooping);
    notifyListeners();
    setLooping(val: newLooping);
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
}

// --- EXTENSION FOR COPYWITH (Helper) ---
extension TransportStateCopyWith on TransportState {
  TransportState copyWith({
    bool? isPlaying,
    bool? isRecording,
    bool? isLooping,
    int? playheadPositionSamples,
    int? loopStartSamples,
    int? loopEndSamples,
  }) {
    return TransportState(
      isPlaying: isPlaying ?? this.isPlaying,
      isRecording: isRecording ?? this.isRecording,
      isLooping: isLooping ?? this.isLooping,
      playheadPositionSamples:
          playheadPositionSamples ?? this.playheadPositionSamples,
      loopStartSamples: loopStartSamples ?? this.loopStartSamples,
      loopEndSamples: loopEndSamples ?? this.loopEndSamples,
    );
  }
}
