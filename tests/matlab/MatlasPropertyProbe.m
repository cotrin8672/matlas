classdef MatlasPropertyProbe < handle
    properties (Dependent)
        Value
    end
    methods
        function value = get.Value(~)
            assignin('base', 'matlas_getter_ran', true);
            value = 11;
        end
        function set.Value(~, ~)
            assignin('base', 'matlas_setter_ran', true);
        end
    end
end
