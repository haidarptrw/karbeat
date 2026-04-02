// GENERATED CODE - DO NOT MODIFY BY HAND
// coverage:ignore-file
// ignore_for_file: type=lint
// ignore_for_file: unused_element, deprecated_member_use, deprecated_member_use_from_same_package, use_function_type_syntax_for_parameters, unnecessary_const, avoid_init_to_null, invalid_override_different_default_values_named, prefer_expression_function_bodies, annotate_overrides, invalid_annotation_target, unnecessary_question_mark

part of 'project.dart';

// **************************************************************************
// FreezedGenerator
// **************************************************************************

// dart format off
T _$identity<T>(T value) => value;
/// @nodoc
mixin _$UiClipSource {





@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is UiClipSource);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'UiClipSource()';
}


}

/// @nodoc
class $UiClipSourceCopyWith<$Res>  {
$UiClipSourceCopyWith(UiClipSource _, $Res Function(UiClipSource) __);
}


/// Adds pattern-matching-related methods to [UiClipSource].
extension UiClipSourcePatterns on UiClipSource {
/// A variant of `map` that fallback to returning `orElse`.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case _:
///     return orElse();
/// }
/// ```

@optionalTypeArgs TResult maybeMap<TResult extends Object?>({TResult Function( UiClipSource_Audio value)?  audio,TResult Function( UiClipSource_Midi value)?  midi,TResult Function( UiClipSource_None value)?  none,required TResult orElse(),}){
final _that = this;
switch (_that) {
case UiClipSource_Audio() when audio != null:
return audio(_that);case UiClipSource_Midi() when midi != null:
return midi(_that);case UiClipSource_None() when none != null:
return none(_that);case _:
  return orElse();

}
}
/// A `switch`-like method, using callbacks.
///
/// Callbacks receives the raw object, upcasted.
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case final Subclass2 value:
///     return ...;
/// }
/// ```

@optionalTypeArgs TResult map<TResult extends Object?>({required TResult Function( UiClipSource_Audio value)  audio,required TResult Function( UiClipSource_Midi value)  midi,required TResult Function( UiClipSource_None value)  none,}){
final _that = this;
switch (_that) {
case UiClipSource_Audio():
return audio(_that);case UiClipSource_Midi():
return midi(_that);case UiClipSource_None():
return none(_that);}
}
/// A variant of `map` that fallback to returning `null`.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case _:
///     return null;
/// }
/// ```

@optionalTypeArgs TResult? mapOrNull<TResult extends Object?>({TResult? Function( UiClipSource_Audio value)?  audio,TResult? Function( UiClipSource_Midi value)?  midi,TResult? Function( UiClipSource_None value)?  none,}){
final _that = this;
switch (_that) {
case UiClipSource_Audio() when audio != null:
return audio(_that);case UiClipSource_Midi() when midi != null:
return midi(_that);case UiClipSource_None() when none != null:
return none(_that);case _:
  return null;

}
}
/// A variant of `when` that fallback to an `orElse` callback.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case _:
///     return orElse();
/// }
/// ```

@optionalTypeArgs TResult maybeWhen<TResult extends Object?>({TResult Function( int sourceId)?  audio,TResult Function( int patternId)?  midi,TResult Function()?  none,required TResult orElse(),}) {final _that = this;
switch (_that) {
case UiClipSource_Audio() when audio != null:
return audio(_that.sourceId);case UiClipSource_Midi() when midi != null:
return midi(_that.patternId);case UiClipSource_None() when none != null:
return none();case _:
  return orElse();

}
}
/// A `switch`-like method, using callbacks.
///
/// As opposed to `map`, this offers destructuring.
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case Subclass2(:final field2):
///     return ...;
/// }
/// ```

@optionalTypeArgs TResult when<TResult extends Object?>({required TResult Function( int sourceId)  audio,required TResult Function( int patternId)  midi,required TResult Function()  none,}) {final _that = this;
switch (_that) {
case UiClipSource_Audio():
return audio(_that.sourceId);case UiClipSource_Midi():
return midi(_that.patternId);case UiClipSource_None():
return none();}
}
/// A variant of `when` that fallback to returning `null`
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case _:
///     return null;
/// }
/// ```

@optionalTypeArgs TResult? whenOrNull<TResult extends Object?>({TResult? Function( int sourceId)?  audio,TResult? Function( int patternId)?  midi,TResult? Function()?  none,}) {final _that = this;
switch (_that) {
case UiClipSource_Audio() when audio != null:
return audio(_that.sourceId);case UiClipSource_Midi() when midi != null:
return midi(_that.patternId);case UiClipSource_None() when none != null:
return none();case _:
  return null;

}
}

}

/// @nodoc


class UiClipSource_Audio extends UiClipSource {
  const UiClipSource_Audio({required this.sourceId}): super._();
  

 final  int sourceId;

/// Create a copy of UiClipSource
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$UiClipSource_AudioCopyWith<UiClipSource_Audio> get copyWith => _$UiClipSource_AudioCopyWithImpl<UiClipSource_Audio>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is UiClipSource_Audio&&(identical(other.sourceId, sourceId) || other.sourceId == sourceId));
}


@override
int get hashCode => Object.hash(runtimeType,sourceId);

@override
String toString() {
  return 'UiClipSource.audio(sourceId: $sourceId)';
}


}

/// @nodoc
abstract mixin class $UiClipSource_AudioCopyWith<$Res> implements $UiClipSourceCopyWith<$Res> {
  factory $UiClipSource_AudioCopyWith(UiClipSource_Audio value, $Res Function(UiClipSource_Audio) _then) = _$UiClipSource_AudioCopyWithImpl;
@useResult
$Res call({
 int sourceId
});




}
/// @nodoc
class _$UiClipSource_AudioCopyWithImpl<$Res>
    implements $UiClipSource_AudioCopyWith<$Res> {
  _$UiClipSource_AudioCopyWithImpl(this._self, this._then);

  final UiClipSource_Audio _self;
  final $Res Function(UiClipSource_Audio) _then;

/// Create a copy of UiClipSource
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? sourceId = null,}) {
  return _then(UiClipSource_Audio(
sourceId: null == sourceId ? _self.sourceId : sourceId // ignore: cast_nullable_to_non_nullable
as int,
  ));
}


}

/// @nodoc


class UiClipSource_Midi extends UiClipSource {
  const UiClipSource_Midi({required this.patternId}): super._();
  

 final  int patternId;

/// Create a copy of UiClipSource
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$UiClipSource_MidiCopyWith<UiClipSource_Midi> get copyWith => _$UiClipSource_MidiCopyWithImpl<UiClipSource_Midi>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is UiClipSource_Midi&&(identical(other.patternId, patternId) || other.patternId == patternId));
}


@override
int get hashCode => Object.hash(runtimeType,patternId);

@override
String toString() {
  return 'UiClipSource.midi(patternId: $patternId)';
}


}

/// @nodoc
abstract mixin class $UiClipSource_MidiCopyWith<$Res> implements $UiClipSourceCopyWith<$Res> {
  factory $UiClipSource_MidiCopyWith(UiClipSource_Midi value, $Res Function(UiClipSource_Midi) _then) = _$UiClipSource_MidiCopyWithImpl;
@useResult
$Res call({
 int patternId
});




}
/// @nodoc
class _$UiClipSource_MidiCopyWithImpl<$Res>
    implements $UiClipSource_MidiCopyWith<$Res> {
  _$UiClipSource_MidiCopyWithImpl(this._self, this._then);

  final UiClipSource_Midi _self;
  final $Res Function(UiClipSource_Midi) _then;

/// Create a copy of UiClipSource
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? patternId = null,}) {
  return _then(UiClipSource_Midi(
patternId: null == patternId ? _self.patternId : patternId // ignore: cast_nullable_to_non_nullable
as int,
  ));
}


}

/// @nodoc


class UiClipSource_None extends UiClipSource {
  const UiClipSource_None(): super._();
  






@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is UiClipSource_None);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'UiClipSource.none()';
}


}




/// @nodoc
mixin _$UiGeneratorInstanceType {





@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is UiGeneratorInstanceType);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'UiGeneratorInstanceType()';
}


}

/// @nodoc
class $UiGeneratorInstanceTypeCopyWith<$Res>  {
$UiGeneratorInstanceTypeCopyWith(UiGeneratorInstanceType _, $Res Function(UiGeneratorInstanceType) __);
}


/// Adds pattern-matching-related methods to [UiGeneratorInstanceType].
extension UiGeneratorInstanceTypePatterns on UiGeneratorInstanceType {
/// A variant of `map` that fallback to returning `orElse`.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case _:
///     return orElse();
/// }
/// ```

@optionalTypeArgs TResult maybeMap<TResult extends Object?>({TResult Function( UiGeneratorInstanceType_Plugin value)?  plugin,TResult Function( UiGeneratorInstanceType_Sampler value)?  sampler,required TResult orElse(),}){
final _that = this;
switch (_that) {
case UiGeneratorInstanceType_Plugin() when plugin != null:
return plugin(_that);case UiGeneratorInstanceType_Sampler() when sampler != null:
return sampler(_that);case _:
  return orElse();

}
}
/// A `switch`-like method, using callbacks.
///
/// Callbacks receives the raw object, upcasted.
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case final Subclass2 value:
///     return ...;
/// }
/// ```

@optionalTypeArgs TResult map<TResult extends Object?>({required TResult Function( UiGeneratorInstanceType_Plugin value)  plugin,required TResult Function( UiGeneratorInstanceType_Sampler value)  sampler,}){
final _that = this;
switch (_that) {
case UiGeneratorInstanceType_Plugin():
return plugin(_that);case UiGeneratorInstanceType_Sampler():
return sampler(_that);}
}
/// A variant of `map` that fallback to returning `null`.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case _:
///     return null;
/// }
/// ```

@optionalTypeArgs TResult? mapOrNull<TResult extends Object?>({TResult? Function( UiGeneratorInstanceType_Plugin value)?  plugin,TResult? Function( UiGeneratorInstanceType_Sampler value)?  sampler,}){
final _that = this;
switch (_that) {
case UiGeneratorInstanceType_Plugin() when plugin != null:
return plugin(_that);case UiGeneratorInstanceType_Sampler() when sampler != null:
return sampler(_that);case _:
  return null;

}
}
/// A variant of `when` that fallback to an `orElse` callback.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case _:
///     return orElse();
/// }
/// ```

@optionalTypeArgs TResult maybeWhen<TResult extends Object?>({TResult Function( UiPluginInstance field0)?  plugin,TResult Function( int assetId,  int rootNote)?  sampler,required TResult orElse(),}) {final _that = this;
switch (_that) {
case UiGeneratorInstanceType_Plugin() when plugin != null:
return plugin(_that.field0);case UiGeneratorInstanceType_Sampler() when sampler != null:
return sampler(_that.assetId,_that.rootNote);case _:
  return orElse();

}
}
/// A `switch`-like method, using callbacks.
///
/// As opposed to `map`, this offers destructuring.
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case Subclass2(:final field2):
///     return ...;
/// }
/// ```

@optionalTypeArgs TResult when<TResult extends Object?>({required TResult Function( UiPluginInstance field0)  plugin,required TResult Function( int assetId,  int rootNote)  sampler,}) {final _that = this;
switch (_that) {
case UiGeneratorInstanceType_Plugin():
return plugin(_that.field0);case UiGeneratorInstanceType_Sampler():
return sampler(_that.assetId,_that.rootNote);}
}
/// A variant of `when` that fallback to returning `null`
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case _:
///     return null;
/// }
/// ```

@optionalTypeArgs TResult? whenOrNull<TResult extends Object?>({TResult? Function( UiPluginInstance field0)?  plugin,TResult? Function( int assetId,  int rootNote)?  sampler,}) {final _that = this;
switch (_that) {
case UiGeneratorInstanceType_Plugin() when plugin != null:
return plugin(_that.field0);case UiGeneratorInstanceType_Sampler() when sampler != null:
return sampler(_that.assetId,_that.rootNote);case _:
  return null;

}
}

}

/// @nodoc


class UiGeneratorInstanceType_Plugin extends UiGeneratorInstanceType {
  const UiGeneratorInstanceType_Plugin(this.field0): super._();
  

 final  UiPluginInstance field0;

/// Create a copy of UiGeneratorInstanceType
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$UiGeneratorInstanceType_PluginCopyWith<UiGeneratorInstanceType_Plugin> get copyWith => _$UiGeneratorInstanceType_PluginCopyWithImpl<UiGeneratorInstanceType_Plugin>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is UiGeneratorInstanceType_Plugin&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode => Object.hash(runtimeType,field0);

@override
String toString() {
  return 'UiGeneratorInstanceType.plugin(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $UiGeneratorInstanceType_PluginCopyWith<$Res> implements $UiGeneratorInstanceTypeCopyWith<$Res> {
  factory $UiGeneratorInstanceType_PluginCopyWith(UiGeneratorInstanceType_Plugin value, $Res Function(UiGeneratorInstanceType_Plugin) _then) = _$UiGeneratorInstanceType_PluginCopyWithImpl;
@useResult
$Res call({
 UiPluginInstance field0
});




}
/// @nodoc
class _$UiGeneratorInstanceType_PluginCopyWithImpl<$Res>
    implements $UiGeneratorInstanceType_PluginCopyWith<$Res> {
  _$UiGeneratorInstanceType_PluginCopyWithImpl(this._self, this._then);

  final UiGeneratorInstanceType_Plugin _self;
  final $Res Function(UiGeneratorInstanceType_Plugin) _then;

/// Create a copy of UiGeneratorInstanceType
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(UiGeneratorInstanceType_Plugin(
null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as UiPluginInstance,
  ));
}


}

/// @nodoc


class UiGeneratorInstanceType_Sampler extends UiGeneratorInstanceType {
  const UiGeneratorInstanceType_Sampler({required this.assetId, required this.rootNote}): super._();
  

 final  int assetId;
 final  int rootNote;

/// Create a copy of UiGeneratorInstanceType
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$UiGeneratorInstanceType_SamplerCopyWith<UiGeneratorInstanceType_Sampler> get copyWith => _$UiGeneratorInstanceType_SamplerCopyWithImpl<UiGeneratorInstanceType_Sampler>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is UiGeneratorInstanceType_Sampler&&(identical(other.assetId, assetId) || other.assetId == assetId)&&(identical(other.rootNote, rootNote) || other.rootNote == rootNote));
}


@override
int get hashCode => Object.hash(runtimeType,assetId,rootNote);

@override
String toString() {
  return 'UiGeneratorInstanceType.sampler(assetId: $assetId, rootNote: $rootNote)';
}


}

/// @nodoc
abstract mixin class $UiGeneratorInstanceType_SamplerCopyWith<$Res> implements $UiGeneratorInstanceTypeCopyWith<$Res> {
  factory $UiGeneratorInstanceType_SamplerCopyWith(UiGeneratorInstanceType_Sampler value, $Res Function(UiGeneratorInstanceType_Sampler) _then) = _$UiGeneratorInstanceType_SamplerCopyWithImpl;
@useResult
$Res call({
 int assetId, int rootNote
});




}
/// @nodoc
class _$UiGeneratorInstanceType_SamplerCopyWithImpl<$Res>
    implements $UiGeneratorInstanceType_SamplerCopyWith<$Res> {
  _$UiGeneratorInstanceType_SamplerCopyWithImpl(this._self, this._then);

  final UiGeneratorInstanceType_Sampler _self;
  final $Res Function(UiGeneratorInstanceType_Sampler) _then;

/// Create a copy of UiGeneratorInstanceType
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? assetId = null,Object? rootNote = null,}) {
  return _then(UiGeneratorInstanceType_Sampler(
assetId: null == assetId ? _self.assetId : assetId // ignore: cast_nullable_to_non_nullable
as int,rootNote: null == rootNote ? _self.rootNote : rootNote // ignore: cast_nullable_to_non_nullable
as int,
  ));
}


}

// dart format on
