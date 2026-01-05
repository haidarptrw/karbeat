// GENERATED CODE - DO NOT MODIFY BY HAND
// coverage:ignore-file
// ignore_for_file: type=lint
// ignore_for_file: unused_element, deprecated_member_use, deprecated_member_use_from_same_package, use_function_type_syntax_for_parameters, unnecessary_const, avoid_init_to_null, invalid_override_different_default_values_named, prefer_expression_function_bodies, annotate_overrides, invalid_annotation_target, unnecessary_question_mark

part of 'session.dart';

// **************************************************************************
// FreezedGenerator
// **************************************************************************

// dart format off
T _$identity<T>(T value) => value;
/// @nodoc
mixin _$UiClipboardContent {





@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is UiClipboardContent);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'UiClipboardContent()';
}


}

/// @nodoc
class $UiClipboardContentCopyWith<$Res>  {
$UiClipboardContentCopyWith(UiClipboardContent _, $Res Function(UiClipboardContent) __);
}


/// Adds pattern-matching-related methods to [UiClipboardContent].
extension UiClipboardContentPatterns on UiClipboardContent {
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

@optionalTypeArgs TResult maybeMap<TResult extends Object?>({TResult Function( UiClipboardContent_Empty value)?  empty,TResult Function( UiClipboardContent_Notes value)?  notes,TResult Function( UiClipboardContent_Clips value)?  clips,required TResult orElse(),}){
final _that = this;
switch (_that) {
case UiClipboardContent_Empty() when empty != null:
return empty(_that);case UiClipboardContent_Notes() when notes != null:
return notes(_that);case UiClipboardContent_Clips() when clips != null:
return clips(_that);case _:
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

@optionalTypeArgs TResult map<TResult extends Object?>({required TResult Function( UiClipboardContent_Empty value)  empty,required TResult Function( UiClipboardContent_Notes value)  notes,required TResult Function( UiClipboardContent_Clips value)  clips,}){
final _that = this;
switch (_that) {
case UiClipboardContent_Empty():
return empty(_that);case UiClipboardContent_Notes():
return notes(_that);case UiClipboardContent_Clips():
return clips(_that);}
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

@optionalTypeArgs TResult? mapOrNull<TResult extends Object?>({TResult? Function( UiClipboardContent_Empty value)?  empty,TResult? Function( UiClipboardContent_Notes value)?  notes,TResult? Function( UiClipboardContent_Clips value)?  clips,}){
final _that = this;
switch (_that) {
case UiClipboardContent_Empty() when empty != null:
return empty(_that);case UiClipboardContent_Notes() when notes != null:
return notes(_that);case UiClipboardContent_Clips() when clips != null:
return clips(_that);case _:
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

@optionalTypeArgs TResult maybeWhen<TResult extends Object?>({TResult Function()?  empty,TResult Function( List<UiNote> field0)?  notes,TResult Function( List<UiClip> field0)?  clips,required TResult orElse(),}) {final _that = this;
switch (_that) {
case UiClipboardContent_Empty() when empty != null:
return empty();case UiClipboardContent_Notes() when notes != null:
return notes(_that.field0);case UiClipboardContent_Clips() when clips != null:
return clips(_that.field0);case _:
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

@optionalTypeArgs TResult when<TResult extends Object?>({required TResult Function()  empty,required TResult Function( List<UiNote> field0)  notes,required TResult Function( List<UiClip> field0)  clips,}) {final _that = this;
switch (_that) {
case UiClipboardContent_Empty():
return empty();case UiClipboardContent_Notes():
return notes(_that.field0);case UiClipboardContent_Clips():
return clips(_that.field0);}
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

@optionalTypeArgs TResult? whenOrNull<TResult extends Object?>({TResult? Function()?  empty,TResult? Function( List<UiNote> field0)?  notes,TResult? Function( List<UiClip> field0)?  clips,}) {final _that = this;
switch (_that) {
case UiClipboardContent_Empty() when empty != null:
return empty();case UiClipboardContent_Notes() when notes != null:
return notes(_that.field0);case UiClipboardContent_Clips() when clips != null:
return clips(_that.field0);case _:
  return null;

}
}

}

/// @nodoc


class UiClipboardContent_Empty extends UiClipboardContent {
  const UiClipboardContent_Empty(): super._();
  






@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is UiClipboardContent_Empty);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'UiClipboardContent.empty()';
}


}




/// @nodoc


class UiClipboardContent_Notes extends UiClipboardContent {
  const UiClipboardContent_Notes(final  List<UiNote> field0): _field0 = field0,super._();
  

 final  List<UiNote> _field0;
 List<UiNote> get field0 {
  if (_field0 is EqualUnmodifiableListView) return _field0;
  // ignore: implicit_dynamic_type
  return EqualUnmodifiableListView(_field0);
}


/// Create a copy of UiClipboardContent
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$UiClipboardContent_NotesCopyWith<UiClipboardContent_Notes> get copyWith => _$UiClipboardContent_NotesCopyWithImpl<UiClipboardContent_Notes>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is UiClipboardContent_Notes&&const DeepCollectionEquality().equals(other._field0, _field0));
}


@override
int get hashCode => Object.hash(runtimeType,const DeepCollectionEquality().hash(_field0));

@override
String toString() {
  return 'UiClipboardContent.notes(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $UiClipboardContent_NotesCopyWith<$Res> implements $UiClipboardContentCopyWith<$Res> {
  factory $UiClipboardContent_NotesCopyWith(UiClipboardContent_Notes value, $Res Function(UiClipboardContent_Notes) _then) = _$UiClipboardContent_NotesCopyWithImpl;
@useResult
$Res call({
 List<UiNote> field0
});




}
/// @nodoc
class _$UiClipboardContent_NotesCopyWithImpl<$Res>
    implements $UiClipboardContent_NotesCopyWith<$Res> {
  _$UiClipboardContent_NotesCopyWithImpl(this._self, this._then);

  final UiClipboardContent_Notes _self;
  final $Res Function(UiClipboardContent_Notes) _then;

/// Create a copy of UiClipboardContent
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(UiClipboardContent_Notes(
null == field0 ? _self._field0 : field0 // ignore: cast_nullable_to_non_nullable
as List<UiNote>,
  ));
}


}

/// @nodoc


class UiClipboardContent_Clips extends UiClipboardContent {
  const UiClipboardContent_Clips(final  List<UiClip> field0): _field0 = field0,super._();
  

 final  List<UiClip> _field0;
 List<UiClip> get field0 {
  if (_field0 is EqualUnmodifiableListView) return _field0;
  // ignore: implicit_dynamic_type
  return EqualUnmodifiableListView(_field0);
}


/// Create a copy of UiClipboardContent
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$UiClipboardContent_ClipsCopyWith<UiClipboardContent_Clips> get copyWith => _$UiClipboardContent_ClipsCopyWithImpl<UiClipboardContent_Clips>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is UiClipboardContent_Clips&&const DeepCollectionEquality().equals(other._field0, _field0));
}


@override
int get hashCode => Object.hash(runtimeType,const DeepCollectionEquality().hash(_field0));

@override
String toString() {
  return 'UiClipboardContent.clips(field0: $field0)';
}


}

/// @nodoc
abstract mixin class $UiClipboardContent_ClipsCopyWith<$Res> implements $UiClipboardContentCopyWith<$Res> {
  factory $UiClipboardContent_ClipsCopyWith(UiClipboardContent_Clips value, $Res Function(UiClipboardContent_Clips) _then) = _$UiClipboardContent_ClipsCopyWithImpl;
@useResult
$Res call({
 List<UiClip> field0
});




}
/// @nodoc
class _$UiClipboardContent_ClipsCopyWithImpl<$Res>
    implements $UiClipboardContent_ClipsCopyWith<$Res> {
  _$UiClipboardContent_ClipsCopyWithImpl(this._self, this._then);

  final UiClipboardContent_Clips _self;
  final $Res Function(UiClipboardContent_Clips) _then;

/// Create a copy of UiClipboardContent
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? field0 = null,}) {
  return _then(UiClipboardContent_Clips(
null == field0 ? _self._field0 : field0 // ignore: cast_nullable_to_non_nullable
as List<UiClip>,
  ));
}


}

// dart format on
