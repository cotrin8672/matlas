function run_v06_tests
% Exercise fixed handler arity and the v0.6 scalar/callback conveniences.
fprintf('MATLAS_STAGE v06_fixed_arity\n');
expectError({}, 1, 'matlas:input:invalid');
expectError({true, false}, 1, 'matlas:input:invalid');
% Numeric inputs would raise the custom ID if the handler ran.
expectError({42}, 0, 'matlas:input:invalid');
expectError({42}, 2, 'matlas:input:invalid');

fprintf('MATLAS_STAGE v06_text\n');
withNul = ['left' char(0) 'right'];
texts = {'日本語😀', "日本語😀", '', "", char(zeros(1, 0)), ...
    withNul, string(withNul)};
for i = 1:numel(texts)
    actual = matlas_fixed_integration(texts{i});
    expected = char(texts{i});
    assert(ischar(actual));
    assert(isequal(uint16(actual(:)), uint16(expected(:))));
end
rejectedText = {['ab'; 'cd'], ['a'; 'b'], reshape('abcd', 1, 2, 2), ...
    char(zeros(0, 3)), ["first", "second"], strings(0, 0), {'text'}};
for i = 1:numel(rejectedText)
    expectError(rejectedText(i), 1, 'matlas:array:type');
end
expectError({string(missing)}, 1, 'matlas:input:invalid');
expectError({char(55296)}, 1, 'matlas:input:invalid'); % Unpaired UTF-16 surrogate.

fprintf('MATLAS_STAGE v06_logical\n');
for value = [false, true]
    actual = matlas_fixed_integration(value);
    assert(islogical(actual) && isscalar(actual) && actual == value);
    actual = matlas_fixed_integration(sparse(value));
    assert(islogical(actual) && isscalar(actual) && ~issparse(actual));
    assert(actual == value);
end
expectError({logical.empty(0, 0)}, 1, 'matlas:array:type');
expectError({[true, false]}, 1, 'matlas:array:type');
expectError({1}, 1, 'matlasTest:CustomInput');
expectError({struct}, 1, 'matlas:array:type');
% A failing conversion or custom error must leave subsequent calls usable.
assert(isequal(matlas_fixed_integration("after errors"), 'after errors'));
fprintf('MATLAS_V06_PASS fixed arity; Unicode/NUL/empty/missing text; dense/sparse logical; custom ID; callback arity\n');
end

function expectError(args, outputCount, identifier)
try
    if outputCount == 0
        matlas_fixed_integration(args{:});
    else
        outputs = cell(1, outputCount);
        [outputs{:}] = matlas_fixed_integration(args{:});
    end
catch e
    assert(strcmp(e.identifier, identifier), '%s: %s', e.identifier, e.message);
    return
end
error('matlas:test:expectedError', 'Expected %s', identifier);
end
