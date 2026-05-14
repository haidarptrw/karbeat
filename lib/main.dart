import 'dart:io';

import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:karbeat/app.dart';
import 'package:karbeat/src/rust/frb_generated.dart';
import 'package:window_manager/window_manager.dart';
import 'package:flutter/services.dart';

Future<void> main() async {
  WidgetsFlutterBinding.ensureInitialized();

  bool isDesktop = !kIsWeb && (Platform.isWindows || Platform.isLinux || Platform.isMacOS);

  if (isDesktop) {
    // Initialize the window manager ONLY on desktop
    await windowManager.ensureInitialized();
    
    // Set the initial title
    await windowManager.setTitle('DigiDAW — Untitled');

    await windowManager.setMinimumSize(const Size(1000, 600));
  await windowManager.setAspectRatio(16 / 9);
  } else {
    if (kIsWeb) {
      throw Exception("This app is not compatible with Web");
    }
    await SystemChrome.setPreferredOrientations([
      DeviceOrientation.landscapeLeft,
      DeviceOrientation.landscapeRight,
    ]);

      SystemChrome.setSystemUIOverlayStyle(const SystemUiOverlayStyle(
      statusBarColor: Colors.transparent, // Makes the status bar see-through
      statusBarIconBrightness: Brightness.light, // Use 'dark' if your background is light
      systemNavigationBarColor: Colors.transparent, // Optional: for bottom bar
    ));

    // This line tells the app to extend its layout behind system bars
    SystemChrome.setEnabledSystemUIMode(SystemUiMode.edgeToEdge);
  }
  

  await RustLib.init();

  runApp(const ProviderScope(child: KarbeatApp()));
}
