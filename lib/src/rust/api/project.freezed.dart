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
mixin _$AudioExportConfigDTO {

 Object get field0;



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is AudioExportConfigDTO&&const DeepCollectionEquality().equals(other.field0, field0));
}


@override
int get hashCode => Object.hash(runtimeType,const DeepCollectionEquality().hash(field0));

@override
String toString() {
  return 'AudioExportConfigDTO(field0: $field0)';
}


}

/// @nodoc
class $AudioExportConfigDTOCopyWith<$Res>  {
$AudioExportConfigDTOCopyWith(AudioExportConfigDTO _, $Res Function(AudioExportConfigDTO) __);
}


/// Adds pattern-matching-related methods to [AudioExportConfigDTO].
extension AudioExportConfigDTOPatterns on AudioExportConfigDTO {
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

@optionalTypeArgs TResult maybeMap<TResult extends Object?>({TResult Function( AudioExportConfigDTO_Wav value)?  wav,TResult Function( AudioExportConfigDTO_Mp3 value)?  mp3,required TResult orElse(),}){
final _that = this;
switch (_that) {
case AudioExportConfigDTO_Wav() when wav != null:
return wav(_that);case AudioExportConfigDTO_Mp3() when mp3 != null:
return mp3(_that);case _:
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

@optionalTypeArgs TResult map<TResult extends Object?>({required TResult Function( AudioExportConfigDTO_Wav value)  wav,required TResult Function( AudioExportConfigDTO_Mp3 value)  mp3,}){
final _that = this;
switch (_that) {
case AudioExportConfigDTO_Wav():
return wav(_that);case AudioExportConfigDTO_Mp3():
return mp3(_that);}
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

@optionalTypeArgs TResult? mapOrNull<TResult extends Object?>({TResult? Function( AudioExportConfigDTO_Wav value)?  wav,TResult? Function( AudioExportConfigDTO_Mp3 value)?  mp3,}){
final _that = this;
switch (_that) {
case AudioExportConfigDTO_Wav() when wav != null:
return wav(_that);case AudioExportConfigDTO_Mp3() when mp3 != null:
return mp3(_that);case _:
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

@optionalTypeArgs TResult maybeWhen<TResult extends Object?>({TResult Function( WavExportConfigDTO field0)?  wav,TResult Function( Mp3ExportConfigDTO field0)?  mp3,required TResult orElse(),}) {final _that = this;
switch (_that) {
case AudioExportConfigDTO_Wav() when wav != null:
return wav(_that.field0);case AudioExportConfigDTO_Mp3() when mp3 != null:
return mp3(_that.field0);case _:
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

@optionalTypeArgs TResult when<TResult extends Object?>({required TResult Function( WavExportConfigDTO field0)  wav,required TResult Function( Mp3ExportConfigDTO field0)  mp3,}) {final _that = this;
switch (_that) {
case AudioExportConfigDTO_Wav():
return wav(_that.field0);case AudioExportConfigDTO_Mp3():
return mp3(_that.field0);}
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

@optionalTypeArgs TResult? whenOrNull<TResult extends Object?>({TResult? Function( WavExportConfigDTO field0)?  wav,TResult? Function( Mp3ExportConfigDTO field0)?  mp3,}) {final _that = this;
switch (_that) {
case AudioExportConfigDTO_Wav() when wav != null:
return wav(_that.field0);case AudioExportConfigDTO_Mp3() when mp3 != null:
return mp3(_that.field0);case _:
  return null;

}
}

}

/// @nodoc


class AudioExportConfigDTO_Wav extends AudioExportConfigDTO {
  const AudioExportConfigDTO_Wav(this.field0): super._();
  

@override final  WavExportConfigDTO field0;

/// Create a copy of AudioExportConfigDTO
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$AudioExportConfigDTO_WavCopyWith<AudioExportConfigDTO_Wav> get copyWith => _$AudioExportConfigDTO_WavCopyWithImpl<AudioExportConfigDTO_Wav>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is AudioExportConfigDTO_Wav&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode => Object.hash(runtimeType,field0);

@override
String toString() {
  return 'AudioExportConfigDTO.wav(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $AudioExportConfigDTO_WavCopyWith<$Res> implements $AudioExportConfigDTOCopyWith<$Res> {
  factory $AudioExportConfigDTO_WavCopyWith(AudioExportConfigDTO_Wav value, $Res Function(AudioExportConfigDTO_Wav) _then) = _$AudioExportConfigDTO_WavCopyWithImpl;
@useResult
$Res call({
 WavExportConfigDTO field0
});




}
/// @nodoc
class _$AudioExportConfigDTO_WavCopyWithImpl<$Res>
    implements $AudioExportConfigDTO_WavCopyWith<$Res> {
  _$AudioExportConfigDTO_WavCopyWithImpl(this._self, this._then);

  final AudioExportConfigDTO_Wav _self;
  final $Res Function(AudioExportConfigDTO_Wav) _then;

/// Create a copy of AudioExportConfigDTO
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(AudioExportConfigDTO_Wav(
null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as WavExportConfigDTO,
  ));
}


}

/// @nodoc


class AudioExportConfigDTO_Mp3 extends AudioExportConfigDTO {
  const AudioExportConfigDTO_Mp3(this.field0): super._();
  

@override final  Mp3ExportConfigDTO field0;

/// Create a copy of AudioExportConfigDTO
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$AudioExportConfigDTO_Mp3CopyWith<AudioExportConfigDTO_Mp3> get copyWith => _$AudioExportConfigDTO_Mp3CopyWithImpl<AudioExportConfigDTO_Mp3>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is AudioExportConfigDTO_Mp3&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode => Object.hash(runtimeType,field0);

@override
String toString() {
  return 'AudioExportConfigDTO.mp3(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $AudioExportConfigDTO_Mp3CopyWith<$Res> implements $AudioExportConfigDTOCopyWith<$Res> {
  factory $AudioExportConfigDTO_Mp3CopyWith(AudioExportConfigDTO_Mp3 value, $Res Function(AudioExportConfigDTO_Mp3) _then) = _$AudioExportConfigDTO_Mp3CopyWithImpl;
@useResult
$Res call({
 Mp3ExportConfigDTO field0
});




}
/// @nodoc
class _$AudioExportConfigDTO_Mp3CopyWithImpl<$Res>
    implements $AudioExportConfigDTO_Mp3CopyWith<$Res> {
  _$AudioExportConfigDTO_Mp3CopyWithImpl(this._self, this._then);

  final AudioExportConfigDTO_Mp3 _self;
  final $Res Function(AudioExportConfigDTO_Mp3) _then;

/// Create a copy of AudioExportConfigDTO
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(AudioExportConfigDTO_Mp3(
null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as Mp3ExportConfigDTO,
  ));
}


}

/// @nodoc
mixin _$BitDepthDTO {

 int get field0;
/// Create a copy of BitDepthDTO
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$BitDepthDTOCopyWith<BitDepthDTO> get copyWith => _$BitDepthDTOCopyWithImpl<BitDepthDTO>(this as BitDepthDTO, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is BitDepthDTO&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode => Object.hash(runtimeType,field0);

@override
String toString() {
  return 'BitDepthDTO(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $BitDepthDTOCopyWith<$Res>  {
  factory $BitDepthDTOCopyWith(BitDepthDTO value, $Res Function(BitDepthDTO) _then) = _$BitDepthDTOCopyWithImpl;
@useResult
$Res call({
 int field0
});




}
/// @nodoc
class _$BitDepthDTOCopyWithImpl<$Res>
    implements $BitDepthDTOCopyWith<$Res> {
  _$BitDepthDTOCopyWithImpl(this._self, this._then);

  final BitDepthDTO _self;
  final $Res Function(BitDepthDTO) _then;

/// Create a copy of BitDepthDTO
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') @override $Res call({Object? field0 = null,}) {
  return _then(_self.copyWith(
field0: null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as int,
  ));
}

}


/// Adds pattern-matching-related methods to [BitDepthDTO].
extension BitDepthDTOPatterns on BitDepthDTO {
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

@optionalTypeArgs TResult maybeMap<TResult extends Object?>({TResult Function( BitDepthDTO_BitPerSample value)?  bitPerSample,TResult Function( BitDepthDTO_BitPerSecond value)?  bitPerSecond,required TResult orElse(),}){
final _that = this;
switch (_that) {
case BitDepthDTO_BitPerSample() when bitPerSample != null:
return bitPerSample(_that);case BitDepthDTO_BitPerSecond() when bitPerSecond != null:
return bitPerSecond(_that);case _:
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

@optionalTypeArgs TResult map<TResult extends Object?>({required TResult Function( BitDepthDTO_BitPerSample value)  bitPerSample,required TResult Function( BitDepthDTO_BitPerSecond value)  bitPerSecond,}){
final _that = this;
switch (_that) {
case BitDepthDTO_BitPerSample():
return bitPerSample(_that);case BitDepthDTO_BitPerSecond():
return bitPerSecond(_that);}
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

@optionalTypeArgs TResult? mapOrNull<TResult extends Object?>({TResult? Function( BitDepthDTO_BitPerSample value)?  bitPerSample,TResult? Function( BitDepthDTO_BitPerSecond value)?  bitPerSecond,}){
final _that = this;
switch (_that) {
case BitDepthDTO_BitPerSample() when bitPerSample != null:
return bitPerSample(_that);case BitDepthDTO_BitPerSecond() when bitPerSecond != null:
return bitPerSecond(_that);case _:
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

@optionalTypeArgs TResult maybeWhen<TResult extends Object?>({TResult Function( int field0)?  bitPerSample,TResult Function( int field0)?  bitPerSecond,required TResult orElse(),}) {final _that = this;
switch (_that) {
case BitDepthDTO_BitPerSample() when bitPerSample != null:
return bitPerSample(_that.field0);case BitDepthDTO_BitPerSecond() when bitPerSecond != null:
return bitPerSecond(_that.field0);case _:
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

@optionalTypeArgs TResult when<TResult extends Object?>({required TResult Function( int field0)  bitPerSample,required TResult Function( int field0)  bitPerSecond,}) {final _that = this;
switch (_that) {
case BitDepthDTO_BitPerSample():
return bitPerSample(_that.field0);case BitDepthDTO_BitPerSecond():
return bitPerSecond(_that.field0);}
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

@optionalTypeArgs TResult? whenOrNull<TResult extends Object?>({TResult? Function( int field0)?  bitPerSample,TResult? Function( int field0)?  bitPerSecond,}) {final _that = this;
switch (_that) {
case BitDepthDTO_BitPerSample() when bitPerSample != null:
return bitPerSample(_that.field0);case BitDepthDTO_BitPerSecond() when bitPerSecond != null:
return bitPerSecond(_that.field0);case _:
  return null;

}
}

}

/// @nodoc


class BitDepthDTO_BitPerSample extends BitDepthDTO {
  const BitDepthDTO_BitPerSample(this.field0): super._();
  

@override final  int field0;

/// Create a copy of BitDepthDTO
/// with the given fields replaced by the non-null parameter values.
@override @JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$BitDepthDTO_BitPerSampleCopyWith<BitDepthDTO_BitPerSample> get copyWith => _$BitDepthDTO_BitPerSampleCopyWithImpl<BitDepthDTO_BitPerSample>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is BitDepthDTO_BitPerSample&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode => Object.hash(runtimeType,field0);

@override
String toString() {
  return 'BitDepthDTO.bitPerSample(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $BitDepthDTO_BitPerSampleCopyWith<$Res> implements $BitDepthDTOCopyWith<$Res> {
  factory $BitDepthDTO_BitPerSampleCopyWith(BitDepthDTO_BitPerSample value, $Res Function(BitDepthDTO_BitPerSample) _then) = _$BitDepthDTO_BitPerSampleCopyWithImpl;
@override @useResult
$Res call({
 int field0
});




}
/// @nodoc
class _$BitDepthDTO_BitPerSampleCopyWithImpl<$Res>
    implements $BitDepthDTO_BitPerSampleCopyWith<$Res> {
  _$BitDepthDTO_BitPerSampleCopyWithImpl(this._self, this._then);

  final BitDepthDTO_BitPerSample _self;
  final $Res Function(BitDepthDTO_BitPerSample) _then;

/// Create a copy of BitDepthDTO
/// with the given fields replaced by the non-null parameter values.
@override @pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(BitDepthDTO_BitPerSample(
null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as int,
  ));
}


}

/// @nodoc


class BitDepthDTO_BitPerSecond extends BitDepthDTO {
  const BitDepthDTO_BitPerSecond(this.field0): super._();
  

@override final  int field0;

/// Create a copy of BitDepthDTO
/// with the given fields replaced by the non-null parameter values.
@override @JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$BitDepthDTO_BitPerSecondCopyWith<BitDepthDTO_BitPerSecond> get copyWith => _$BitDepthDTO_BitPerSecondCopyWithImpl<BitDepthDTO_BitPerSecond>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is BitDepthDTO_BitPerSecond&&(identical(other.field0, field0) || other.field0 == field0));
}


@override
int get hashCode => Object.hash(runtimeType,field0);

@override
String toString() {
  return 'BitDepthDTO.bitPerSecond(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $BitDepthDTO_BitPerSecondCopyWith<$Res> implements $BitDepthDTOCopyWith<$Res> {
  factory $BitDepthDTO_BitPerSecondCopyWith(BitDepthDTO_BitPerSecond value, $Res Function(BitDepthDTO_BitPerSecond) _then) = _$BitDepthDTO_BitPerSecondCopyWithImpl;
@override @useResult
$Res call({
 int field0
});




}
/// @nodoc
class _$BitDepthDTO_BitPerSecondCopyWithImpl<$Res>
    implements $BitDepthDTO_BitPerSecondCopyWith<$Res> {
  _$BitDepthDTO_BitPerSecondCopyWithImpl(this._self, this._then);

  final BitDepthDTO_BitPerSecond _self;
  final $Res Function(BitDepthDTO_BitPerSecond) _then;

/// Create a copy of BitDepthDTO
/// with the given fields replaced by the non-null parameter values.
@override @pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(BitDepthDTO_BitPerSecond(
null == field0 ? _self.field0 : field0 // ignore: cast_nullable_to_non_nullable
as int,
  ));
}


}

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
