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
    fprintf('RUSTMAT_STAGE format=%d write\n', version);
    assert(rustmat_integration(1, version, data{:}) == 0);
    verifyFormat('roundtrip.mat', version);
    loaded = load('roundtrip.mat');
    verify(loaded, data);
    fprintf('RUSTMAT_STAGE format=%d info\n', version);
    assert(rustmat_integration(3, version, data{:}) == 0);
    fprintf('RUSTMAT_STAGE format=%d info_iterator\n', version);
    assert(rustmat_integration(6, version, data{:}) == 0);
    actual = cell(size(data));
    fprintf('RUSTMAT_STAGE format=%d value_iterator\n', version);
    [actual{:}] = rustmat_integration(5, version);
    for i = 1:numel(data), assert(isequaln(actual{i}, data{i})); end

    original = struct;
    for i = 1:numel(data), original.(sprintf('v%d', i-1)) = data{i}; end
    save('matlab.mat', '-struct', 'original', saveFlags{version + 1});
    fprintf('RUSTMAT_STAGE format=%d MATLAB_to_Rust\n', version);
    [actual{:}] = rustmat_integration(2, version);
    for i = 1:numel(data)
        assert(isequaln(actual{i}, data{i}));
        assert(strcmp(class(actual{i}), class(data{i})));
        assert(isequal(size(actual{i}), size(data{i})));
        assert(issparse(actual{i}) == issparse(data{i}));
    end

    fprintf('RUSTMAT_STAGE format=%d update\n', version);
    assert(rustmat_integration(4, version, 99) == 0);
    updated = load('roundtrip.mat');
    assert(updated.v0 == 99 && ~isfield(updated, 'v1'));
    fprintf('RUSTMAT_STAGE format=%d failures_empty\n', version);
    assert(rustmat_integration(10, version) == 0);
    assert(~isfile('nonexistent.mat'));
    if version >= 3
        assert(rustmat_integration(7, version, [3 4]) == 0);
        clear global global_value
        load('global.mat');
        globalInfo = whos('global_value');
        assert(globalInfo.global);
        assert(isequal(global_value, [3 4]));
        clear global global_value
        unicode = rustmat_integration(9, version, complex([1 2], [3 4]));
        assert(isequal(unicode, complex([1 2], [3 4])));
        unicodeFile = load(fullfile('日本語😀', '値😀.mat'));
        assert(isequal(unicodeFile.value, unicode));
    end
    checks = checks + numel(data) * 3;
    fprintf('RUSTMAT_FORMAT_PASS %d (%d arrays)\n', version, numel(data));
end

try
    result = rustmat_integration(8, 3, [7 8]); %#ok<NASGU>
    error('rustmat:test:noError', 'Expected an error');
catch e
    assert(strcmp(e.identifier, 'rustmat:input:invalid'), e.message);
end
early = load('early.mat');
assert(isequal(early.saved, [7 8]));
movefile('early.mat', 'early-closed.mat', 'f');
try
    result = rustmat_integration(12, 3, [9 10]); %#ok<NASGU>
    error('rustmat:test:noPanic', 'Expected a panic error');
catch e
    assert(strcmp(e.identifier, 'rustmat:mex:panic'), e.message);
end
panicked = load('panic.mat');
assert(isequal(panicked.saved, [9 10]));
movefile('panic.mat', 'panic-closed.mat', 'f');
try
    rustmat_integration(); % NULL/zero-count input and output arrays
    error('rustmat:test:noArgsError', 'Expected an argument error');
catch e
    assert(strcmp(e.identifier, 'rustmat:test:args'), e.message);
end
try
    rustmat_integration(13, 3);
    error('rustmat:test:noBufferError', 'Expected a formatted error');
catch e
    assert(strcmp(e.identifier, 'rustmat:mex:error'), e.message);
    assert(contains(e.message, '日本語 %s 100%?tail'), e.message);
end
rustmat_integration(1, 3, basic{:}); % successful zero-output invocation
from_caller = [10 20 30];
assert(rustmat_integration(11, 3) == 0);
caller = load('caller.mat');
assert(isequal(caller.from_caller, from_caller));
for repeat = 1:25
    assert(rustmat_integration(1, 3, basic{:}) == 0);
    out = cell(size(basic));
    [out{:}] = rustmat_integration(5, 3);
    assert(isequal(out, basic));
end
fprintf('RUSTMAT_ALL_PASS %d array-direction checks; 5 formats; lifecycle/global/Unicode/workspace\n', checks);
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
