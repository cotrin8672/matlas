classdef MatlasDropMutator < handle
    methods
        function delete(~)
            count = evalin('base', 'matlas_drop_count');
            assignin('base', 'matlas_drop_count', count + 1);
            assignin('base', 'matlas_handoff_input', -1);
        end
    end
end
