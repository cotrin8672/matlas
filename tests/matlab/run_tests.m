function run_tests(repo)
% Exercise actual MATLAB -> Rust MEX -> libmat paths in both directions.
arguments
    repo (1,1) string
end
addpath(fullfile(repo, 'target', 'debug'));
testdir = fullfile(repo, 'work', 'integration');
if ~isfolder(testdir), mkdir(testdir); end
old = pwd;
restore = onCleanup(@() cd(old)); %#ok<NASGU>
cd(testdir);
if ~isfolder('日本語😀'), mkdir('日本語😀'); end
fid = fopen('corrupt.mat', 'w');
fwrite(fid, 'not a MAT-file');
fclose(fid);

basic = {reshape(1:6, 2, 3), complex([1 2; 3 4], [5 6; 7 8])};
values = [basic, {single([1 NaN Inf]), complex(single([1 2]), single([3 4])), ...
    int8([-128 127]), uint8([0 255]), int16([-32768 32767]), uint16([0 65535]), ...
    int32([-7 8]), uint32([7 8]), int64([-7 8]), uint64([7 8]), ...
    logical([0 1; 1 0]), 'Hello 日本語', [], zeros(0, 3), reshape(1:24, 2, 3, 4), ...
    sparse([1 0; 0 2]), sparse(complex([1 0; 0 2], [0 3; 0 0])), sparse(logical(eye(2))), ...
    {42, 'text'}, struct('field', reshape(1:6, 2, 3), 'nested', {{42}}), cell(0, 2)}];
saveFlags = {'-v6', '-v4', '-v6', '-v7', '-v7.3'};
checks = 0;
for version = 0:4
    data = values;
    if version <= 2, data = basic; end
    fprintf('MATLAS_STAGE format=%d write\n', version);
    assert(matlas_integration(1, version, data{:}) == 0);
    verifyFormat('roundtrip.mat', version);
    loaded = load('roundtrip.mat');
    verify(loaded, data);
    fprintf('MATLAS_STAGE format=%d info\n', version);
    assert(matlas_integration(3, version, data{:}) == 0);
    fprintf('MATLAS_STAGE format=%d info_iterator\n', version);
    assert(matlas_integration(6, version, data{:}) == 0);
    actual = cell(size(data));
    fprintf('MATLAS_STAGE format=%d value_iterator\n', version);
    [actual{:}] = matlas_integration(5, version);
    for i = 1:numel(data), assert(isequaln(actual{i}, data{i})); end

    original = struct;
    for i = 1:numel(data), original.(sprintf('v%d', i-1)) = data{i}; end
    save('matlab.mat', '-struct', 'original', saveFlags{version + 1});
    fprintf('MATLAS_STAGE format=%d MATLAB_to_Rust\n', version);
    [actual{:}] = matlas_integration(2, version);
    for i = 1:numel(data)
        assert(isequaln(actual{i}, data{i}));
        assert(strcmp(class(actual{i}), class(data{i})));
        assert(isequal(size(actual{i}), size(data{i})));
        assert(issparse(actual{i}) == issparse(data{i}));
    end

    fprintf('MATLAS_STAGE format=%d update\n', version);
    assert(matlas_integration(4, version, 99) == 0);
    updated = load('roundtrip.mat');
    assert(updated.v0 == 99 && ~isfield(updated, 'v1'));
    fprintf('MATLAS_STAGE format=%d failures_empty\n', version);
    assert(matlas_integration(10, version) == 0);
    assert(~isfile('nonexistent.mat'));
    if version >= 3
        assert(matlas_integration(7, version, [3 4]) == 0);
        clear global global_value
        load('global.mat');
        globalInfo = whos('global_value');
        assert(globalInfo.global);
        assert(isequal(global_value, [3 4]));
        clear global global_value
        unicode = matlas_integration(9, version, complex([1 2], [3 4]));
        assert(isequal(unicode, complex([1 2], [3 4])));
        unicodeFile = load(fullfile('日本語😀', '値😀.mat'));
        assert(isequal(unicodeFile.value, unicode));
    end
    checks = checks + numel(data) * 3;
    fprintf('MATLAS_FORMAT_PASS %d (%d arrays)\n', version, numel(data));
end
fprintf('MATLAS_STAGE callback_after_formats\n');
assert(matlas_integration(21, 3, 2, 5) == 7);

try
    result = matlas_integration(8, 3, [7 8]); %#ok<NASGU>
    error('matlas:test:noError', 'Expected an error');
catch e
    assert(strcmp(e.identifier, 'matlas:input:invalid'), e.message);
end
early = load('early.mat');
assert(isequal(early.saved, [7 8]));
movefile('early.mat', 'early-closed.mat', 'f');
try
    result = matlas_integration(12, 3, [9 10]); %#ok<NASGU>
    error('matlas:test:noPanic', 'Expected a panic error');
catch e
    assert(strcmp(e.identifier, 'matlas:panic'), e.message);
end
panicked = load('panic.mat');
assert(isequal(panicked.saved, [9 10]));
movefile('panic.mat', 'panic-closed.mat', 'f');
try
    matlas_integration(); % NULL/zero-count input and output arrays
    error('matlas:test:noArgsError', 'Expected an argument error');
catch e
    assert(strcmp(e.identifier, 'matlas:input:invalid'), e.message);
end
try
    matlas_integration(13, 3);
    error('matlas:test:noBufferError', 'Expected a formatted error');
catch e
    assert(strcmp(e.identifier, 'matlas:native'), e.message);
    assert(contains(e.message, '日本語 %s 100%?tail'), e.message);
end
fprintf('MATLAS_STAGE callback_after_errors\n');
assert(matlas_integration(21, 3, 2, 5) == 7);
matlas_integration(1, 3, basic{:}); % successful zero-output invocation
from_caller = [10 20 30];
assert(matlas_integration(11, 3) == 0);
caller = load('caller.mat');
assert(isequal(caller.from_caller, from_caller));
for repeat = 1:25
    assert(matlas_integration(1, 3, basic{:}) == 0);
    out = cell(size(basic));
    [out{:}] = matlas_integration(5, 3);
    assert(isequal(out, basic));
end
fprintf('MATLAS_STAGE callback_after_repeats\n');
assert(matlas_integration(21, 3, 2, 5) == 7);
fprintf('MATLAS_STAGE ownership_api\n');
[madeNumeric, madeString, madeLogical, madeCell, madeSparse] = matlas_integration(14, 3);
assert(isequal(madeNumeric, [1 3; 2 4]));
assert(strcmp(madeString, '日本語'));
assert(isequal(madeLogical, logical([1 0; 0 1])));
assert(isequal(madeCell, {42}));
assert(isequal(madeSparse, sparse([5 0; 0 6])));
[madeLogicalSparse, mutatedSparse, linearIndex, complexSparse, charMatrix, emptySparse, emptyLogicalSparse] = matlas_integration(15, 3, sparse([1 0; 0 2]));
assert(isequal(madeLogicalSparse, sparse(logical(eye(2)))));
assert(isequal(mutatedSparse, sparse([5 0; 0 9])));
assert(linearIndex == 3); % zero-based linear index for zero-based [1, 1]
assert(isequal(complexSparse, sparse([1+2i 0; 0 7+4i])));
assert(isequal(charMatrix, ['ab'; 'c ']));
assert(isequal(size(emptySparse), [0 3]) && issparse(emptySparse));
assert(isequal(size(emptyLogicalSparse), [0 3]) && issparse(emptyLogicalSparse) && islogical(emptyLogicalSparse));
fprintf('MATLAS_STAGE callback_after_ownership\n');
assert(matlas_integration(21, 3, 2, 5) == 7);
fprintf('MATLAS_STAGE persistent_cross_invocation\n');
assert(matlas_integration(16, 3, 123.5) == 0);
assert(matlas_integration(17, 3) == 123.5);
assert(matlas_integration(17, 3) == 123.5);
assert(matlas_integration(18, 3) == 0);
try
    matlas_integration(17, 3);
    error('matlas:test:noPersistentError', 'Expected a missing persistent value error');
catch e
    assert(strcmp(e.identifier, 'matlas:input:invalid'), e.message);
end
fprintf('MATLAS_STAGE callback_after_persistent\n');
assert(matlas_integration(21, 3, 2, 5) == 7);
fprintf('MATLAS_STAGE module_lock_raii\n');
assert(matlas_integration(19, 3) == 1);
assert(matlas_integration(1, 3, basic{:}) == 0);
assert(matlas_integration(20, 3) == 0);
fprintf('MATLAS_STAGE callback_after_lock\n');
assert(matlas_integration(21, 3, 2, 5) == 7);
fprintf('MATLAS_STAGE trapped_callbacks\n');
assert(matlas_integration(21, 3, 2, 5) == 7);
assert(numel(matlas_integration(24, 3)) == 6);
assert(matlas_integration(25, 3) == 0);
[emptyStruct, madeStruct] = matlas_integration(26, 3);
assert(isstruct(emptyStruct) && isempty(fieldnames(emptyStruct)));
assert(isequal(fieldnames(madeStruct), {'kept'; 'extra'}));
assert(madeStruct.kept == 42 && strcmp(madeStruct.extra, 'ok'));
converted = matlas_integration(27, 3);
assert(isequal(size(converted), [1 4]) && isequal(converted, [1 2 3 4]) && isreal(converted));
from_caller_scalar = 9;
assert(matlas_integration(28, 3) == 10);
assert(from_rust == 10);
second_caller_scalar = 5;
assert(matlas_integration(30, 3) == 14);
assert(matlas_integration(31, 3, 2) == 11);
assert(matlas_integration(32, 3) == 5);
object = MatlasTestObject;
object.Value = 11;
assert(matlas_integration(29, 3, object) == 22);
fprintf('MATLAS_STAGE guarded_workspace_mat\n');
object = MatlasCallbackObject;
save('callback.mat', 'object');
assignin('base', 'matlas_borrowed', 42);
assignin('base', 'matlas_callback_ran', false);
assert(matlas_integration(33, 3, object, {object}, struct('item', object), "text", ...
    logical([1 0]), 'abc', sparse([1 0; 0 2])) == 0);
assert(~evalin('base', 'matlas_callback_ran'));
assert(evalin('base', 'matlas_borrowed') == 42);
plain = load('guarded.mat');
assert(plain.plain == 42);
assert(isequal(plain.logical, logical([1 0])));
assert(strcmp(plain.character, 'abc'));
assert(isequal(plain.sparse, sparse([1 0; 0 2])));
evalin('base', 'clear matlas_borrowed matlas_callback_ran');
try
    matlas_integration(22, 3);
    error('matlas:test:noCallbackError', 'Expected a trapped call error');
catch e
    assert(strcmp(e.identifier, 'matlas:callback'), e.message);
    assert(contains(e.message, 'intentional callback failure'), e.message);
end
try
    matlas_integration(23, 3);
    error('matlas:test:noEvalError', 'Expected a trapped eval error');
catch e
    assert(strcmp(e.identifier, 'matlas:callback'), e.message);
    assert(contains(e.message, 'intentional eval failure'), e.message);
end
run_v06_tests;
fprintf('MATLAS_STAGE v08_plain\n');
plainCandidates = [values, {"text", strings(0, 0), struct, MatlasTestObject, @sin}];
for i = 1:numel(plainCandidates)
    plain_candidate = plainCandidates{i};
    expected = isnumeric(plain_candidate) || islogical(plain_candidate) || ischar(plain_candidate);
    assert(matlas_integration(34, 3, expected) == 0);
end
fprintf('MATLAS_STAGE v08_error_context\n');
try
    matlas_integration(35, 3);
    error('matlas:test:noContextError', 'Expected a contextual callback error');
catch e
    assert(strcmp(e.identifier, 'matlasTest:Context'), e.message);
    assert(contains(e.message, sprintf('outer context\n  inner context\n  ')), e.message);
    assert(contains(e.message, 'call MATLAB'), e.message);
    assert(contains(e.message, 'matlasTest:OriginalCause: original callback failure'), e.message);
end
assert(isequal(matlas_fixed_integration("after context error"), 'after context error'));
fprintf('MATLAS_V08_PASS %d plain classifications; typed ID; nested context; original callback cause\n', numel(plainCandidates));
fprintf('MATLAS_STAGE v09_scalar_property_destroy\n');
assert(matlas_integration(36, 3) == 0);
assignin('base', 'matlas_getter_ran', false);
assignin('base', 'matlas_setter_ran', false);
assert(matlas_integration(37, 3, MatlasPropertyProbe) == 0);
assert(~evalin('base', 'matlas_getter_ran'));
assert(evalin('base', 'matlas_setter_ran'));
evalin('base', 'clear matlas_getter_ran matlas_setter_ran');
for command = 38:40
    assignin('base', 'matlas_drop_count', 0);
    assignin('base', 'matlas_handoff_input', 42);
    if command == 38
        assert(matlas_integration(command, 3) == 43);
    else
        assert(matlas_integration(command, 3) == 0);
    end
    assert(evalin('base', 'matlas_drop_count') == 1);
    assert(evalin('base', 'matlas_handoff_input') == -1);
end
evalin('base', 'clear matlas_drop_count matlas_handoff_input');
fprintf('MATLAS_V09_PASS strict scalar; property setter; deferred destruction\n');
fprintf('MATLAS_STAGE trapped_property_setter\n');
assignin('base', 'matlas_setter_calls', 0);
assert(matlas_integration(41, 3, MatlasThrowingSetter) == 0);
assert(evalin('base', 'matlas_setter_calls') == 1);
evalin('base', 'clear matlas_setter_calls');
objects(1) = MatlasTestObject;
objects(1).Value = 11;
objects(2) = MatlasTestObject;
objects(2).Value = 33;
assert(matlas_integration(42, 3, objects) == 0);
assert(isequal([objects.Value], [11 33]));
fprintf('MATLAS_PROPERTY_SETTER_PASS caught exception; returned value-class array\n');
fprintf('MATLAS_STAGE persistent_child_replacement\n');
for kind = 0:2
    assert(matlas_integration(43, 3, kind) == 0);
    assert(matlas_integration(44, 3, kind, 17) == 0);
    assert(matlas_integration(45, 3, kind) == 17);
    assert(matlas_integration(44, 3, kind, 23) == 17);
    assert(matlas_integration(45, 3, kind) == 23);
    if kind == 1, assert(matlas_integration(46, 3) == 0); end
    assert(matlas_integration(18, 3) == 0);
end
object = MatlasTestObject;
object.Value = 17;
assert(matlas_integration(43, 3, 0) == 0);
assert(matlas_integration(44, 3, 0, object) == 0);
replacement = MatlasTestObject;
replacement.Value = 23;
previous = matlas_integration(44, 3, 0, replacement);
assert(previous.Value == 17);
current = matlas_integration(45, 3, 0);
assert(current.Value == 23);
assert(matlas_integration(18, 3) == 0);
assert(matlas_integration(43, 3, 0) == 0);
assert(matlas_integration(44, 3, 0, MatlasPropertyProbe) == 0);
previous = matlas_integration(44, 3, 0, MatlasPropertyProbe);
assert(isa(previous, 'MatlasPropertyProbe'));
assert(matlas_integration(18, 3) == 0);
fprintf('MATLAS_PERSISTENT_CHILD_PASS cell; struct; nested cell; value and handle objects\n');
fprintf('MATLAS_STAGE context_input_output_contracts\n');
assert(matlas_integration(47, 3) == 42);
contextFile = load('context.mat');
assert(contextFile.x == 21 && contextFile.y == 42);
assert(matlas_integration(16, 3, 21) == 0);
assert(matlas_integration(48, 3) == 42);
assert(matlas_integration(18, 3) == 0);
clear ans;
matlas_integration(49, 3);
assert(exist('ans', 'var') == 1 && ans == 42);
assert(matlas_integration(49, 3) == 42);
matlas_integration(50, 3);
try
    missing = matlas_integration(50, 3); %#ok<NASGU>
    error('matlas:test:expectedMissingOutput', 'Expected a missing output error');
catch e
    assert(strcmp(e.identifier, 'matlas:input:invalid'), e.message);
    assert(contains(e.message, 'requested output 0 was not set'), e.message);
end
try
    [first, second] = matlas_integration(51, 3); %#ok<ASGLU>
    error('matlas:test:expectedMissingOutput', 'Expected a missing output error');
catch e
    assert(strcmp(e.identifier, 'matlas:input:invalid'), e.message);
    assert(contains(e.message, 'requested output 1 was not set'), e.message);
end
assert(matlas_integration(52, 3) == 1);
fprintf('MATLAS_API_CONTRACT_PASS context callback; persistent read; ans; requested outputs\n');
fprintf('MATLAS_ALL_PASS %d array-direction checks; 5 formats; lifecycle/global/Unicode/workspace/persistent/lock-RAII/callbacks/v0.10\n', checks);
end

function verifyFormat(path, version)
fid = fopen(path, 'r', 'ieee-le');
closeFile = onCleanup(@() fclose(fid)); %#ok<NASGU>
header = fread(fid, 128, '*uint8')';
switch version
    case 1
        fseek(fid, 0, 'bof');
        descriptor = fread(fid, 5, 'int32')';
        assert(isequal(descriptor, [0 2 3 0 3])); % v4 double v0, 2x3
    case {2, 3}
        assert(startsWith(char(header), 'MATLAB 5.0 MAT-file'));
        tag = fread(fid, 1, 'uint32');
        expected = 14; % miMATRIX (v6, uncompressed)
        if version == 3, expected = 15; end % miCOMPRESSED (v7)
        assert(tag == expected);
    case 4
        assert(startsWith(char(header), 'MATLAB 7.3 MAT-file'));
        fseek(fid, 512, 'bof');
        assert(isequal(fread(fid, 8, '*uint8')', uint8([137 72 68 70 13 10 26 10])));
end
end

function verify(actual, expected)
assert(numel(fieldnames(actual)) == numel(expected));
for i = 1:numel(expected)
    value = actual.(sprintf('v%d', i-1));
    assert(isequaln(value, expected{i}));
    assert(strcmp(class(value), class(expected{i})));
    assert(isequal(size(value), size(expected{i})));
    assert(issparse(value) == issparse(expected{i}));
end
end
