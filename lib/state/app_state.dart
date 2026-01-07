import 'dart:async';
import 'dart:developer';

import 'package:flutter/foundation.dart';
import 'package:karbeat/models/grid.dart';
import 'package:karbeat/models/menu_group.dart';
import 'package:karbeat/src/rust/api/audio.dart';
import 'package:karbeat/src/rust/api/pattern.dart';
import 'package:karbeat/src/rust/api/plugin.dart';
import 'package:karbeat/src/rust/api/project.dart';
import 'package:karbeat/src/rust/api/track.dart';
import 'package:karbeat/src/rust/api/track.dart' as track_api;
import 'package:karbeat/src/rust/api/transport.dart' as transport_api;
import 'package:karbeat/src/rust/audio/event.dart';
import 'package:karbeat/src/rust/core/project.dart';
import 'package:karbeat/src/rust/api/session.dart' as session_api;
import 'package:karbeat/src/rust/core/project/track.dart';
import 'package:karbeat/src/rust/core/project/transport.dart';
import 'package:karbeat/utils/formatter.dart';
import 'package:karbeat/utils/logger.dart';

enum ToolSelection { pointer, cut, draw, move, delete, scrub, zoom, select }

enum WorkspaceView { trackList, pianoRoll, mixer, source }

enum ToolbarMenuContextGroup { none, project, edit, view }

/// Events that trigger a state refresh
enum ProjectEvent {
  tracksChanged,
  transportChanged,
  metadataChanged,
  sourceListChanged,
  generatorListChanged,
  sessionChanged,
  configChanged,
  patternChanged,
}

class KarbeatState extends ChangeNotifier {
  // ================== BACKEND STATES =========================
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
    sampleRate: 48000,
    bufferSize: 256,
    cpuLoad: 0,
  );

  // List<Clipboard>

  // =================== STORES ==========================
  Map<int, UiTrack> _tracks = {};
  Map<int, AudioWaveformUiForAudioProperties> _audioSources = {};
  Map<int, UiGeneratorInstance> _generators = {};
  Map<int, UiPattern> _patterns = {};
  UiSessionState? _sessionState;
  List<String> _availableGenerators = [];
  List<String> get availableGenerators => _availableGenerators;

  static final List<KarbeatToolbarMenuGroup> menuGroups = [
    KarbeatToolbarMenuGroupFactory.createProjectMenuGroup(),
    KarbeatToolbarMenuGroupFactory.createEditMenuGroup(),
    KarbeatToolbarMenuGroupFactory.createViewMenuGroup(),
  ];

  int maxSamplesIndex = 2000;

  late final Stream<PlaybackPosition> _positionBroadcastStream;

  // STRATEGY: Internal Event Bus for State Synchronization
  final StreamController<ProjectEvent> _stateEventController =
      StreamController.broadcast();

  // ignore:unused_field
  StreamSubscription<ProjectEvent>? _stateSubscription;

  // =========== EDITOR STATE ====================
  ToolSelection _selectedTool = ToolSelection.pointer;
  WorkspaceView _currentView = WorkspaceView.trackList;
  ToolbarMenuContextGroup _currentToolbarContext = ToolbarMenuContextGroup.none;
  int _piannoRollGridDenom = 4;
  int? _editingPatternId;

  /// Denominator of the grid size (e.g 4 = 1/4 note, 16 = 1/16 note)
  int gridSize = 4;

  // ================== OTHER STATES ====================
  bool _pendingPlayRequest = false;

  // ================ CONSTRUCTOR ==================
  KarbeatState() {
    _positionBroadcastStream = createPositionStream().asBroadcastStream();
    _initStateListener();
    _positionBroadcastStream.listen((pos) {
      if (pos.isPlaying) {
        _pendingPlayRequest = false;
      }

      // Only react if the state has actually changed
      if (pos.isPlaying != _transportState.isPlaying) {
        if (_pendingPlayRequest && !pos.isPlaying) {
          return;
        }

        // Update UI
        _transportState = _transportState.copyWith(isPlaying: pos.isPlaying);
        notifyListeners();

        if (!pos.isPlaying) {
          transport_api.setPlaying(val: false);
        }
      }
    });
    syncTrackState();
    syncMaxSampleIndex();
    syncTransportState();
    syncMetadataState();
    syncAudioSourceList();
    syncPatternList();
    syncGeneratorList();
    syncAudioHardwareConfigState();

    // fetch available generators
    fetchAvailableGenerators();
  }

  Future<void> fetchAvailableGenerators() async {
    try {
      final list = await getAvailableGenerators();
      _availableGenerators = list;
      notifyListeners();
    } catch (e) {
      log("Error fetching plugins: $e");
    }
  }

  void _initStateListener() {
    _stateSubscription = _stateEventController.stream.listen((event) async {
      switch (event) {
        case ProjectEvent.tracksChanged:
          await syncTrackState();
          await syncMaxSampleIndex();
          break;
        case ProjectEvent.transportChanged:
          await syncTransportState();
          break;
        case ProjectEvent.metadataChanged:
          await syncMetadataState();
          break;
        case ProjectEvent.sourceListChanged:
          await syncAudioSourceList();
          break;
        case ProjectEvent.generatorListChanged:
          await syncGeneratorList();
          break;
        case ProjectEvent.configChanged:
          await syncAudioHardwareConfigState();
          break;
        case ProjectEvent.sessionChanged:
          await syncSessionState();
          break;
        case ProjectEvent.patternChanged:
          await syncPatternList();
          break;
      }
    });
  }

  // ============== GETTERS =================
  TransportState get transport => _transportState;
  ProjectMetadata get metadata => _metadata;
  bool get isPlaying => _transportState.isPlaying;
  bool get isLooping => _transportState.isLooping;
  double get tempo => _transportState.bpm;
  Map<int, UiTrack> get tracks => _tracks;
  Map<int, AudioWaveformUiForAudioProperties> get audioSources => _audioSources;
  Map<int, UiGeneratorInstance> get generators => _generators;
  ToolSelection get selectedTool => _selectedTool;
  WorkspaceView get currentView => _currentView;
  ToolbarMenuContextGroup get currentToolbarContext => _currentToolbarContext;
  AudioHardwareConfig get hardwareConfig => _hardwareConfig;
  Stream<PlaybackPosition> get positionStream => _positionBroadcastStream;
  UiSessionState? get sessionState => _sessionState;
  Map<int, UiPattern> get patterns => _patterns;
  int get pianoRollGridDenom => _piannoRollGridDenom;
  int? get editingPatternId => _editingPatternId;

  // ================ SETTERS ===================
  set pianoRollGridDenom(GridValue val) {
    _piannoRollGridDenom = val.value;
  }

  // =============== GLOBAL UI STATE ==========================
  double horizontalZoomLevel = 1000;
  Map<int, int> trackIdHeightMap = {};

  // =============== PLACEMENT MODE STATE (USED WHEN AUDIO CLIP PLACEMENT) =====================
  int? _placingSourceId;
  UiSourceType? _placingSourceType;
  int? get placingSourceId => _placingSourceId;

  // Track where the user wants to drop it
  double _placementTimeSamples = 0.0;
  int _placementTrackId = -1;

  bool get isPlacing => _placingSourceId != null;

  // ================ SYNCHRONIZATION ======================

  /// Syncs only the track state
  Future<void> syncTrackState() async {
    try {
      final newState = await getTracks();
      _tracks = newState;
      notifyListeners();
    } catch (e) {
      KarbeatLogger.error("error when syncing the track state: $e");
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
      KarbeatLogger.error("Transport sync failed: $e");
    }
  }

  /// Syncs only the metadata (Project name, BPM, Time signature)
  Future<void> syncMetadataState() async {
    try {
      final newState = await getProjectMetadata();
      _metadata = newState;
      notifyListeners();
    } catch (e) {
      KarbeatLogger.error("failed when syncing metadata: $e");
    }
  }

  Future<void> syncMaxSampleIndex() async {
    try {
      final newState = await getMaxSampleIndex();
      maxSamplesIndex = newState;
      notifyListeners();
    } catch (e) {
      KarbeatLogger.error("failed when syncing max sample index: $e");
    }
  }

  /// Syncs the list of loaded audio files
  /// Call this when: Adding a file, Removing a file
  Future<void> syncAudioSourceList() async {
    final sources = await getAudioSourceList();
    if (sources != null) {
      _audioSources = Map.from(sources);
      notifyListeners();
    }
  }

  Future<void> syncGeneratorList() async {
    try {
      final generators = await getGeneratorList();
      _generators = generators;
      notifyListeners();
    } catch (e) {
      KarbeatLogger.error("Failed to sync generators: $e");
    }
  }

  Future<void> syncSessionState() async {
    try {
      final newState = await getSessionState();
      _sessionState = newState;
      notifyListeners();
    } catch (e) {
      KarbeatLogger.error("Failed to sync session state: $e");
    }
  }

  Future<void> syncAudioHardwareConfigState() async {
    try {
      final newState = await getAudioConfig();
      _hardwareConfig = newState;
      notifyListeners();
    } catch (e) {
      KarbeatLogger.error("Failed to sync audio hardware state: $e");
    }
  }

  Future<void> syncPatternList() async {
    try {
      final result = await getPatterns();
      _patterns = result;
      notifyListeners();
    } catch (e) {
      KarbeatLogger.error("Failed to sync pattern list: $e");
    }
  }

  /// Efficiently syncs a SINGLE pattern instead of the whole list
  Future<void> syncPattern(int patternId) async {
    try {
      final updatedPattern = await getPattern(patternId: patternId);

      // Creating a new map reference ensures Selectors in UI will trigger a rebuild
      final newMap = Map<int, UiPattern>.from(_patterns);
      newMap[patternId] = updatedPattern;
      _patterns = newMap;

      notifyListeners();
    } catch (e) {
      KarbeatLogger.error("Error syncing single pattern $patternId: $e");
    }
  }

  /// Efficiently syncs a SINGLE track (and its clips)
  Future<void> syncTrack(int trackId) async {
    try {
      final updatedTrack = await getTrack(trackId: trackId);
      final newMap = Map<int, UiTrack>.from(_tracks);
      newMap[trackId] = updatedTrack;
      _tracks = newMap;

      notifyListeners();
    } catch (e) {
      KarbeatLogger.error("Error syncing single track $trackId: $e");
    }
  }

  /// Triggers a state refresh. Call this after any Rust API action.
  void notifyBackendChange(ProjectEvent event) {
    if (!_stateEventController.isClosed) {
      _stateEventController.add(event);
    }
  }

  // =============== ACTIONS ===============

  /// Loads an audio file and refreshes the source list
  Future<void> addAudioFile(String path) async {
    await addAudioSource(filePath: path);
    notifyBackendChange(ProjectEvent.sourceListChanged);
  }

  Future<void> addTrack(TrackType type) async {
    try {
      await addNewTrack(trackType: type);
      notifyBackendChange(ProjectEvent.tracksChanged);
    } catch (e) {
      KarbeatLogger.error("Failed to add track: $e");
    }
  }

  Future<void> addMidiTrackWithGenerator(String generatorName) async {
    try {
      await track_api.addMidiTrackWithGenerator(generatorName: generatorName);
      notifyBackendChange(ProjectEvent.tracksChanged);
      notifyBackendChange(ProjectEvent.generatorListChanged);
    } catch (e) {
      KarbeatLogger.error("Failed to add midi track: $e");
    }
  }

  Future<void> togglePlay() async {
    try {
      final newPlaying = !_transportState.isPlaying;

      // FIX: Set the flag if we are attempting to play
      if (newPlaying) {
        _pendingPlayRequest = true;
      }

      await transport_api.setPlaying(val: newPlaying);
      notifyBackendChange(ProjectEvent.transportChanged);
    } catch (e) {
      log("Failed to toggle play: $e");
      // Reset flag on error
      _pendingPlayRequest = false;
    }
  }

  Future<void> stop() async {
    try {
      await transport_api.setPlaying(val: false);
      await transport_api.setPlayhead(val: 0);
      notifyBackendChange(ProjectEvent.transportChanged);
    } catch (e) {
      KarbeatLogger.error("Failed to stop play: $e");
    }
  }

  Future<void> toggleLoop() async {
    try {
      final newLooping = !_transportState.isLooping;
      await transport_api.setLooping(val: newLooping);
      await syncTransportState();
    } catch (e) {
      KarbeatLogger.error("Failed to toggle loop: $e");
    }
  }

  /// Sets the BPM.
  /// Updates local state optimistically and calls the backend API.
  Future<void> setBpm(double value) async {
    try {
      _transportState = _transportState.copyWith(bpm: value);
      notifyListeners();

      await transport_api.setBpm(val: value);
      notifyBackendChange(ProjectEvent.transportChanged);
    } catch (e) {
      KarbeatLogger.error("Failed to set bpm: $e");
    }
  }

  /// Change the selected tool to a desired tool
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

  /// Opens the piano roll view with a specific pattern (from source list).
  void openPattern(int patternId) {
    _editingPatternId = patternId;
    navigateTo(WorkspaceView.pianoRoll);
  }

  void setGridSize(int newSize) {
    if (gridSize != newSize) {
      gridSize = newSize;
      notifyListeners();
    }
  }

  void setHorizontalZoom(double level) {
    if (horizontalZoomLevel != level) {
      horizontalZoomLevel = level;
      notifyListeners();
    }
  }

  Future<void> seekTo(int samples) async {
    try {
      // Call the Rust API
      await transport_api.setPlayhead(val: samples);

      // Optimistic update (optional, since Rust pushes the update back immediately)
      notifyListeners();
    } catch (e) {
      KarbeatLogger.error("Error seeking: $e");
    }
  }

  Future<void> deleteClip(int trackId, int clipId) async {
    if (_tracks.containsKey(trackId)) {
      final track = _tracks[trackId]!;
      final updatedClips = track.clips.where((c) => c.id != clipId).toList();

      _tracks = Map.from(_tracks);
      _tracks[trackId] = _copyWithTrack(track, clips: updatedClips);
      notifyListeners();
    }

    try {
      await track_api.deleteClip(trackId: trackId, clipId: clipId);
      // notifyBackendChange(ProjectEvent.tracksChanged);
      await syncTrack(trackId);
    } catch (e) {
      KarbeatLogger.error("Error deleting clip: $e");
      // await syncTrackState();
    }
  }

  Future<void> resizeClip(
    int trackId,
    int clipId,
    ResizeEdge edge,
    int newTime,
  ) async {
    _applyOptimisticResize(trackId, clipId, edge, newTime);
    try {
      await track_api.resizeClip(
        trackId: trackId,
        clipId: clipId,
        edge: edge,
        newTimeVal: newTime,
      );
      await syncTrack(trackId);
    } catch (e) {
      KarbeatLogger.error("Error resizing clip: $e");
      // await syncTrackState();
    }
  }

  Future<void> moveClip(
    int trackId,
    int clipId,
    int newStartTime, {
    int? newTrackId,
  }) async {
    try {
      await track_api.moveClip(
        sourceTrackId: trackId,
        clipId: clipId,
        newStartTime: newStartTime,
        newTrackId: newTrackId,
      );
      // notifyBackendChange(ProjectEvent.tracksChanged);

      // Fetch the old track ID and new track ID
      await syncTrack(trackId);
      if (newTrackId != null && newTrackId != trackId) {
        await syncTrack(newTrackId);
      }
    } catch (e) {
      KarbeatLogger.error("Error moving clip: $e");
      // await syncTrackState();
    }
  }

  Future<void> createEmptyPatternClip({
    required int trackId,
    required int startTime,
  }) async {
    try {
      await createClip(
        sourceType: UiSourceType.midi,
        trackId: trackId,
        startTime: startTime,
      );
      KarbeatLogger.info("New empty pattern clip is successfully created");
      // notifyBackendChange(ProjectEvent.tracksChanged);
      await syncTrack(trackId);
    } catch (e) {
      KarbeatLogger.error("Error when creating new empty pattern clip: $e");
    }
  }

  // ===================== NOTE CHANGE API'S ==========================
  Future<void> previewNote({
    required int trackId,
    required int noteKey,
    required bool isOn,
    int velocity = 0,
  }) async {
    try {
      await playPreviewNote(
        trackId: trackId,
        noteKey: noteKey,
        velocity: velocity,
        isOn: isOn,
      );
      KarbeatLogger.info(
        "Play ${numToMidiKey(noteKey)} with generator from $trackId",
      );
    } catch (e) {
      KarbeatLogger.error("Error previewing note: $e");
    }
  }

  Future<void> addPatternNote({
    required int patternId,
    required int key,
    required int startTick,
    required int duration,
  }) async {
    try {
      await addNote(
        patternId: patternId,
        key: key,
        startTick: startTick,
        duration: duration,
      );
      // notifyBackendChange(ProjectEvent.patternChanged);
      await syncPattern(patternId);
    } catch (e) {
      KarbeatLogger.error("Error adding note: $e");
    }
  }

  Future<void> deletePatternNote({
    required int patternId,
    required int noteId,
  }) async {
    _applyOptimisticNoteDeletion(patternId, noteId);
    try {
      await deleteNote(patternId: patternId, noteId: noteId);
      // notifyBackendChange(ProjectEvent.patternChanged);
      await syncPattern(patternId);
    } catch (e) {
      KarbeatLogger.error("Error deleting note: $e");
      await syncPattern(patternId);
    }
  }

  Future<void> movePatternNote({
    required int patternId,
    required int noteId,
    required int newStartTick,
    required int newKey,
  }) async {
    try {
      await moveNote(
        patternId: patternId,
        noteId: noteId,
        newStartTick: newStartTick,
        newKey: newKey,
      );
      // notifyBackendChange(ProjectEvent.patternChanged);
      await syncPattern(patternId);
    } catch (e) {
      KarbeatLogger.error("Error moving note: $e");
    }
  }

  Future<void> resizePatternNote({
    required int patternId,
    required int noteId,
    required int newDuration,
  }) async {
    try {
      await resizeNote(
        patternId: patternId,
        noteId: noteId,
        newDuration: newDuration,
      );
      // notifyBackendChange(ProjectEvent.patternChanged);
      await syncPattern(patternId);
    } catch (e) {
      KarbeatLogger.error("Error resizing note: $e");
    }
  }

  // ==================== OPTIMISTIC HELPERS =============================
  // Helper
  void _applyOptimisticResize(
    int trackId,
    int clipId,
    ResizeEdge edge,
    int newTime,
  ) {
    final track = _tracks[trackId];
    if (track == null) return;

    final clipIndex = track.clips.indexWhere((c) => c.id == clipId);
    if (clipIndex == -1) return;

    final clip = track.clips[clipIndex];

    int newStart = clip.startTime.toInt();
    int newLength = clip.loopLength.toInt();
    int newOffset = clip.offsetStart.toInt();

    if (edge == ResizeEdge.right) {
      // Dragging Right Edge: newTime is the END time
      if (newTime > clip.startTime) {
        newLength = newTime - clip.startTime;
      }
    } else {
      // Dragging Left Edge: newTime is the START time (Slip Edit)
      final oldEnd = clip.startTime + clip.loopLength;

      if (newTime < oldEnd) {
        final delta = newTime - clip.startTime;
        final potentialOffset = clip.offsetStart + delta;

        // Constraint: Offset cannot be negative
        if (potentialOffset >= 0) {
          newStart = newTime;
          newLength = oldEnd - newTime;
          newOffset = potentialOffset.toInt();
        }
      }
    }

    // Create Updated Objects
    final updatedClip = UiClip(
      id: clip.id,
      name: clip.name,
      startTime: newStart,
      loopLength: newLength,
      offsetStart: newOffset,
      source: clip.source,
    );

    final updatedClips = List<UiClip>.from(track.clips);
    updatedClips[clipIndex] = updatedClip;

    final updatedTrack = _copyWithTrack(track, clips: updatedClips);

    _tracks = Map.from(_tracks);
    _tracks[trackId] = updatedTrack;

    notifyListeners();
  }

  void _applyOptimisticNoteDeletion(int patternId, int noteId) {
    final pattern = _patterns[patternId];
    if (pattern == null) return;

    // Filter out the specific note
    final updatedNotes = pattern.notes.where((n) => n.id != noteId).toList();

    final updatedPattern = UiPattern(
      id: pattern.id,
      name: pattern.name,
      lengthTicks: pattern.lengthTicks,
      notes: updatedNotes,
    );

    // Update Store & Notify UI immediately
    final newPatterns = Map<int, UiPattern>.from(_patterns);
    newPatterns[patternId] = updatedPattern;
    _patterns = newPatterns;

    notifyListeners();
  }

  // ============= PLACEMENT MODE LOGIC =================

  void startPlacement(int sourceId, {required UiSourceType type}) {
    _placingSourceId = sourceId;
    _placingSourceType = type;
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
    KarbeatLogger.info("CONFIRM Placement");
    if (_placingSourceId != null &&
        _placingSourceType != null &&
        _placementTrackId != -1) {
      try {
        await createClip(
          sourceId: _placingSourceId!,
          sourceType: _placingSourceType!,
          trackId: _placementTrackId,
          startTime: _placementTimeSamples.toInt(),
        );

        // Reset mode
        _placingSourceId = null;
        _placingSourceType = null;
        _placementTrackId = -1;

        notifyBackendChange(ProjectEvent.tracksChanged);
      } catch (e) {
        KarbeatLogger.error("Error creating clip: $e");
      }
    }
  }

  void cancelPlacement() {
    _placingSourceId = null;
    _placingSourceType = null;
    _placementTrackId = -1;
    notifyListeners();
  }

  UiTrack _copyWithTrack(UiTrack original, {List<UiClip>? clips}) {
    return UiTrack(
      id: original.id,
      name: original.name,
      trackType: original.trackType,
      clips: clips ?? original.clips,
    );
  }

  // ================== Session State public API's =====================

  /// Update the selected clip. ensure the sync state between UI and backend regarding which clip is selected
  Future<void> updateSelectedClip({
    required int trackId,
    required int clipId,
  }) async {
    try {
      await session_api.updateSelectedClip(trackId: trackId, clipId: clipId);
      KarbeatLogger.info(
        "Successfully updated the selected clip to $trackId:$clipId",
      );
      notifyBackendChange(ProjectEvent.sessionChanged);
    } catch (e) {
      KarbeatLogger.error('Error when updating selected clip: $e');
      // await syncSessionState();
    }
  }

  Future<void> deselectClip() async {
    try {
      await session_api.deselectClip();
      notifyBackendChange(ProjectEvent.sessionChanged);
    } catch (e) {
      KarbeatLogger.error('Error when updating selected clip: $e');
      // await syncSessionState();
    }
  }
}

extension TransportStateCopyWith on TransportState {
  TransportState copyWith({
    bool? isPlaying,
    bool? isRecording,
    bool? isLooping,
    int? playheadPositionSamples,
    int? loopStartSamples,
    int? loopEndSamples,
    double? bpm,
    (int, int)? timeSignature,
    int? barTracker,
    int? beatTracker,
  }) {
    return TransportState(
      isPlaying: isPlaying ?? this.isPlaying,
      isRecording: isRecording ?? this.isRecording,
      isLooping: isLooping ?? this.isLooping,
      playheadPositionSamples:
          playheadPositionSamples ?? this.playheadPositionSamples,
      loopStartSamples: loopStartSamples ?? this.loopStartSamples,
      loopEndSamples: loopEndSamples ?? this.loopEndSamples,
      bpm: bpm ?? this.bpm,
      timeSignature: timeSignature ?? this.timeSignature,
      barTracker: barTracker ?? this.barTracker,
      beatTracker: beatTracker ?? this.beatTracker,
    );
  }
}
