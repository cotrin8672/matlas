classdef MatlasCallbackObject
    properties (Dependent)
        Value
    end
    methods
        function value = get.Value(~)
            assignin('base', 'matlas_callback_ran', true);
            assignin('base', 'matlas_borrowed', -1);
            value = 11;
        end
        function saved = saveobj(~)
            assignin('base', 'matlas_callback_ran', true);
            assignin('base', 'matlas_borrowed', -1);
            saved = struct('Value', 11);
        end
    end
    methods (Static)
        function object = loadobj(~)
            assignin('base', 'matlas_callback_ran', true);
            assignin('base', 'matlas_borrowed', -1);
            object = MatlasCallbackObject;
        end
    end
end
