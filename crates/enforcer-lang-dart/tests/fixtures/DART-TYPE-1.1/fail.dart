// FAIL fixture for DART-TYPE-1.1: untyped dynamic DTO parse.
Map<String, dynamic> parse(dynamic json) {
  return json as Map<String, dynamic>;
}
