classdef MatlasThrowingSetter < handle
    properties (Dependent)
        Value
    end
    methods
        function set.Value(~, ~)
            assignin('base', 'matlas_setter_calls', evalin('base', 'matlas_setter_calls') + 1);
            error('matlasTest:SetterFailed', 'setter raised intentionally');
        end
    end
end
