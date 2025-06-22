# Bug Report: Optional Container Field Overwriting

## Issue Summary
The XML parser has a critical scoping bug that causes field values from nested objects to overwrite parent object fields when using Optional container types.

## Failing Test
- **Test**: `python_tests/test_template_union_list.py::test_torture_test_with_actual_parsing`
- **Error**: `AssertionError: assert 'Engineering' == 'TechCorp'`
- **Cause**: Company.name field was overwritten by a nested Department.name field

## Bug Trigger Conditions
The bug occurs when ALL of the following conditions are met:
1. Parent object has a field (e.g., `name: str`)
2. Parent object has an **Optional** container field containing objects with the same field name
3. The parent's field appears **BEFORE** the optional container in the XML

## Affected Types
The bug affects ALL Optional container types:
- ❌ `Optional[List[T]]` - Overwrites parent fields with nested values
- ❌ `Optional[Dict[K, V]]` - Overwrites parent fields with nested values  
- ❌ `Optional[Set[T]]` - Overwrites parent fields with nested values
- ❌ `Optional[Tuple[...]]` - Overwrites parent fields with nested values

## NOT Affected
- ✅ Required containers: `List[T]`, `Dict[K, V]`, `Set[T]`, `Tuple[...]`
- ✅ Optional single objects: `Optional[T]` (non-container)
- ✅ Optional containers when parent field comes AFTER in XML

## Test Results Summary

### Optional[List[Project]]
```python
class Company:
    name: str = "TechCorp"  # Gets overwritten
    projects: Optional[List[Project]]  # Contains Employee → Department.name = "Engineering"
```
Result: Company.name = "Engineering" ❌

### Optional[Dict[str, Employee]]  
```python
class CompanyDict:
    name: str = "TechCorp"  # Gets overwritten
    employee_map: Optional[Dict[str, Employee]]  # Contains Employee → Department.name = "Engineering"
```
Result: Company.name = "Engineering" ❌

### Optional[Set[NamedItem]]
```python
class CompanySet:
    name: str = "TechCorp"  # Gets overwritten
    unique_items: Optional[Set[NamedItem]]  # Contains NamedItem.name = "ItemBeta"
```
Result: Company.name = "ItemBeta" ❌

### Optional[Tuple[Employee, Employee]]
```python
class CompanyTuple:
    name: str = "TechCorp"  # Gets overwritten
    key_employees: Optional[Tuple[Employee, Employee]]  # Second Employee → Department.name = "Sales"
```
Result: Company.name = "Sales" ❌

## Workarounds
1. **Change field order**: Place the parent's field AFTER the optional container in XML
2. **Use required containers**: Remove Optional wrapper if not needed
3. **Rename fields**: Avoid field name collisions between parent and nested objects

## Root Cause
The Rust XML parser is not properly maintaining object scope/context when processing Optional-wrapped containers. During parsing, field assignments from nested objects are incorrectly applied to the parent object's frame instead of their proper context.

The bug appears to be in the stack frame management when entering/exiting Optional container parsing, where the parser loses track of which object should receive field assignments.

## Reproduction Steps
1. Create a class with a field (e.g., `name`)
2. Add an Optional container field with nested objects having the same field name
3. Serialize to XML with the parent field appearing first
4. Parse the XML
5. The parent's field will be overwritten by the last nested object's field value

## Impact
This bug can cause silent data corruption where parent object fields are unexpectedly overwritten by unrelated nested data, leading to incorrect program behavior and test failures.

## Test Files Created
- `debug_torture_exact_failure.py` - Reproduces exact failing test
- `debug_nested_name_overwrite.py` - Tests simple nested structures
- `debug_depth_trigger.py` - Identifies Optional as the trigger
- `debug_optional_trigger.py` - Confirms Optional[List] bug pattern
- `debug_optional_containers.py` - Tests all Optional container types
