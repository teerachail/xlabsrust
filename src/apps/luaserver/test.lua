local contract = {}

function contract.test(p)
    return 'Contract 55: ' .. p.age + 2;
end

return contract;