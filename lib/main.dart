import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:karbeat/app.dart';
import 'package:karbeat/src/rust/frb_generated.dart';
import 'package:window_manager/window_manager.dart';

Future<void> main() async {
  WidgetsFlutterBinding.ensureInitialized();

  // Initialize the window manager
  await windowManager.ensureInitialized();
  
  // Set the initial title
  await windowManager.setTitle('Karbeat — Untitled');

  await RustLib.init();

  runApp(const ProviderScope(child: KarbeatApp()));
}
