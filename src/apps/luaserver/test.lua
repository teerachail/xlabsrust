local contract = {}

function contract.test(p)
    return 'Contract 55: ' .. p.extra;
end

return contract;