import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:mocktail/mocktail.dart';
import 'package:karbeat/features/audio_plugins/effects/karbeat_parametric_eq.dart';
import 'package:karbeat/src/rust/api/plugin.dart' as plugin_api;
import 'package:karbeat/src/rust/frb_generated.dart';

class MockRustLibApi extends Mock implements RustLibApi {}

void main() {
  late MockRustLibApi mockApi;

  setUpAll(() {
    registerFallbackValue(const plugin_api.UiEffectTarget.track(0));
    registerFallbackValue(const plugin_api.UiPluginTarget.masterEffect(0));
  });

  setUp(() {
    mockApi = MockRustLibApi();
    
    // Set up the global mock for the flutter_rust_bridge layer
    RustLib.initMock(api: mockApi);

    // Ignore RenderFlex overflow errors that happen during testing due to limited screen size
    FlutterError.onError = (FlutterErrorDetails details) {
      if (details.exception is FlutterError) {
        final e = details.exception as FlutterError;
        if (e.message.contains('overflowed')) {
          return;
        }
      }
      FlutterError.presentError(details);
    };
  });

  testWidgets('KarbeatParametricEq renders correctly with mocked backend', (WidgetTester tester) async {
    // 1. Stub the API methods called during initState
    
    // getEffectParameterSpecs
    when(() => mockApi.crateApiPluginGetEffectParameterSpecs(
      target: any(named: 'target'),
      effectId: any(named: 'effectId'),
    )).thenAnswer((_) async => [
      const plugin_api.UiPluginParameter(
        id: 100,
        path: 'base_gain',
        name: 'Master Gain',
        group: 'Master',
        value: 0.0,
        min: -24.0,
        max: 24.0,
        defaultValue: 0.0,
        step: 0.1,
        paramType: plugin_api.UiParameterType.float,
        choices: [],
      ),
      const plugin_api.UiPluginParameter(
        id: 0,
        path: 'band0/active',
        name: 'Active',
        group: 'Band 1',
        value: 1.0, // true
        min: 0.0,
        max: 1.0,
        defaultValue: 1.0,
        step: 1.0,
        paramType: plugin_api.UiParameterType.bool,
        choices: [],
      ),
      const plugin_api.UiPluginParameter(
        id: 1,
        path: 'band0/type',
        name: 'Type',
        group: 'Band 1',
        value: 0.0,
        min: 0.0,
        max: 6.0,
        defaultValue: 0.0,
        step: 1.0,
        paramType: plugin_api.UiParameterType.choice,
        choices: ['Peaking', 'LowShelf', 'HighShelf', 'LowPass', 'HighPass', 'BandPass', 'Notch'],
      ),
      const plugin_api.UiPluginParameter(
        id: 2,
        path: 'band0/freq',
        name: 'Frequency',
        group: 'Band 1',
        value: 1000.0,
        min: 20.0,
        max: 20000.0,
        defaultValue: 1000.0,
        step: 1.0,
        paramType: plugin_api.UiParameterType.float,
        choices: [],
      ),
    ]);

    // queryEffectParameters
    when(() => mockApi.crateApiPluginQueryEffectParameters(
      target: any(named: 'target'),
      effectId: any(named: 'effectId'),
    )).thenAnswer((_) async => {});

    // pollEffectParameterFeedback
    when(() => mockApi.crateApiPluginPollEffectParameterFeedback())
      .thenAnswer((_) async => <plugin_api.UiEffectParameterSnapshot>[]);

    // executeRealtimePluginCommand
    when(() => mockApi.crateApiPluginExecuteRealtimePluginCommand(
      target: any(named: 'target'),
      command: any(named: 'command'),
      payloadJson: any(named: 'payloadJson'),
    )).thenAnswer((_) async => 1);

    // createPluginMessageStream
    when(() => mockApi.crateApiPluginCreatePluginMessageStream())
      .thenAnswer((_) => const Stream.empty());

    tester.view.physicalSize = const Size(1920, 1080);
    tester.view.devicePixelRatio = 1.0;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);

    // 2. Build the widget
    await tester.pumpWidget(
      const ProviderScope(
        child: MaterialApp(
          home: Scaffold(
            body: KarbeatParametricEq(
              target: plugin_api.UiEffectTarget.track(0),
              effectId: 42,
            ),
          ),
        ),
      ),
    );

    // Allow Future/Stream builders to resolve
    await tester.pumpAndSettle();

    // 3. Verify elements
    // The widget should render the "MASTER" string for the master gain
    expect(find.text('MASTER'), findsOneWidget);
    
    // It should render "B1" for the first band we mocked
    expect(find.text('B1'), findsOneWidget);
    
    // We didn't provide band 2, but the EQ initializes up to the highest band index found, defaulting to 8 bands
    // Therefore B2 should also exist
    expect(find.text('B2'), findsOneWidget);

    // Consume any RenderFlex overflow exceptions caused by small test window constraints
    while (tester.takeException() != null) {}
  });
}
